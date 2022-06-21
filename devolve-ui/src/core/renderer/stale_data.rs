//! These structs are how devolve-ui communicates to the renderer that it needs to be updated even from another thread.
//! You don't use these directly, instead they are used through classes like `AtomicRefState`.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};
use smallvec::SmallVec;
use crate::core::component::component::VComponent;
use crate::core::component::node::NodeId;
use crate::core::component::path::{VComponentPath, VComponentRefResolved};
use crate::core::component::update_details::UpdateDetails;
use crate::core::misc::is_thread_safe::{TSMutex, TSMutexGuard, TSNotifyFlag};
use crate::core::misc::notify_flag::NotifyFlag;
use crate::core::view::view::VViewData;

/// All data for whether the renderer needs some kind of update.
#[derive(Debug)]
pub(super) struct StaleData<const IS_THREAD_SAFE: bool> {
    /// Components to update
    needs_update: TSMutex<HashMap<VComponentPath, Vec<UpdateDetails>>, IS_THREAD_SAFE>,
    /// Whether we need to rerender.
    /// This should always be set when `needs_update` is modified,
    /// but the invariant is maintained in `Renderer` and not here.
    needs_rerender: TSNotifyFlag<IS_THREAD_SAFE>
}

pub(super) type LocalStaleData = StaleData<false>;
pub(super) type SharedStaleData = StaleData<true>;

/// Error returned from trying to use `StaleData` because of thread poisoning.
/// Not really handled because it's not really expected.
#[derive(Debug)]
pub enum StaleDataError {
    NeedsUpdatePoison
}

pub type StaleDataResult<T> = Result<T, StaleDataError>;

/// Allows you to set that the renderer needs to re-render (no specific components need to update)
/// even from another thread.
#[derive(Debug, Clone)]
pub struct NeedsRerenderFlag(Weak<SharedStaleData>);

/// Allows you to set that a specific component (by path) needs to be updated even from another thread.
#[derive(Debug, Clone)]
pub struct NeedsUpdateFlag {
    stale_data: Weak<SharedStaleData>,
    path: VComponentPath
}

/// Allows you to set that arbitrary components (by path) need to be updated even from another thread.
#[derive(Debug, Clone)]
pub struct NeedsUpdateNotifier(Weak<SharedStaleData>);

impl NeedsRerenderFlag {
    pub(super) fn from(stale_data: &Arc<SharedStaleData>) -> NeedsRerenderFlag {
        NeedsRerenderFlag(Arc::downgrade(stale_data))
    }

    /// Set needs rerender
    pub fn set(&self) {
        if let Some(stale_data) = self.0.upgrade() {
            stale_data.needs_rerender.set();
        }
    }
}

impl NeedsUpdateFlag {
    pub(super) fn from(stale_data: &Arc<SharedStaleData>, path: VComponentPath) -> NeedsUpdateFlag {
        NeedsUpdateFlag {
            stale_data: Arc::downgrade(stale_data),
            path
        }
    }

    /// Flag with empty weak reference which does nothing when set
    pub(in crate::core) fn empty(path: VComponentPath) -> NeedsUpdateFlag {
        NeedsUpdateFlag {
            stale_data: Weak::new(),
            path
        }
    }

    /// Set this component needs update
    pub fn set(&self, details: UpdateDetails) -> StaleDataResult<()> {
        if let Some(stale_data) = self.stale_data.upgrade() {
            stale_data.queue_path_for_update(self.path.clone(), details)?;
            stale_data.needs_rerender.set();
        }
        Ok(())
    }
}

impl NeedsUpdateNotifier {
    pub(super) fn from(stale_data: &Arc<SharedStaleData>) -> NeedsUpdateNotifier {
        NeedsUpdateNotifier(Arc::downgrade(stale_data))
    }

    /// Set this component needs update
    pub fn set(&self, path: VComponentPath, details: UpdateDetails) -> StaleDataResult<()> {
        if let Some(stale_data) = self.0.upgrade() {
            stale_data.queue_path_for_update(path, details)?;
            stale_data.needs_rerender.set();
        }
        Ok(())
    }
}

impl <const IS_THREAD_SAFE: bool> StaleData<IS_THREAD_SAFE> {
    pub(super) fn new() -> Self {
        StaleData {
            needs_update: TSMutex::new(HashMap::with_capacity(4)),
            needs_rerender: TSNotifyFlag::new()
        }
    }

    fn needs_update_lock(&self) -> StaleDataResult<TSMutexGuard<'_, HashMap<VComponentPath, Vec<UpdateDetails>>, IS_THREAD_SAFE>> {
        self.needs_update.lock().map_err(|_err| StaleDataError::NeedsUpdatePoison)
    }

    /// Also should set `needs_rerender` since that is implied by needing to update;
    /// however this function doesn't, `Renderer` which calls this does.
    pub(super) fn queue_path_for_update_with_details(&self, path: &VComponentPath, details: UpdateDetails) -> StaleDataResult<()> {
        let mut needs_update = self.needs_update_lock()?;
        let mut detailss = match needs_update.get_mut(path) {
            None => needs_update.insert(path.clone(), Vec::new()),
            Some(detailss) => detailss
        };
        detailss.push(details);
        Ok(())
    }

    /// Also should set `needs_rerender` since that is implied by needing to update;
    /// however this function doesn't, `Renderer` which calls this does.
    pub(super) fn queue_path_for_update_no_details(&self, path: &VComponentPath) -> StaleDataResult<()> {
        let mut needs_update = self.needs_update_lock()?;
        if needs_update.get(path).is_none() {
            needs_update.insert(path.clone(), Vec::new());
        }
        Ok(())
    }

    /// Transfer all pending updates to components and clear update queue.
    pub(super) fn apply_updates<ViewData: VViewData>(&self, root_component: &mut Box<VComponent<ViewData>>) -> StaleDataResult<()> {
        let mut local_lock = self.needs_update_lock()?;
        for (path, detailss) in local_lock.drain() {
            // Component may no longer exist so we need to check for some
            if let Some(VComponentRefResolved { parent_contexts: _parent_contexts, component: child_component }) = root_component.down_path_mut(&path, Vec::new()) {
                for details in detailss {
                    child_component.head.pending_update(details);
                }
            }
        }
        Ok(())
    }

    /// Access the needs_rerender flag to get, set or clear.
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn needs_rerender<'a>(&'a self) -> &'a TSNotifyFlag<IS_THREAD_SAFE> {
        &self.needs_rerender
    }

    /// Append data from the other stale data and clear it
    pub(super) fn append<const IS_THREAD_SAFE2: bool>(&mut self, other: &mut StaleData<IS_THREAD_SAFE2>) -> StaleDataResult<()> {
        {
            let mut needs_update = self.needs_update_lock()?;
            let mut other_needs_update = other.needs_update_lock();
            for (other_path, other_detailss) in other_needs_update.drain() {
                needs_update.entry(other_path).or_default().append(&mut other_detailss);
            }
        }

        if other.needs_rerender.clear() {
            self.needs_rerender.set();
        }

        Ok(())
    }
}
