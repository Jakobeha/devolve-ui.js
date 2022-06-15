//! These structs are how devolve-ui communicates to the renderer that it needs to be updated even from another thread.
//! You don't use these directly, instead they are used through classes like `AtomicRefState`.
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use smallvec::SmallVec;
use crate::core::component::component::VComponent;
use crate::core::component::node::NodeId;
use crate::core::component::path::VComponentPath;
use crate::core::misc::notify_flag::NotifyFlag;
use crate::core::view::view::VViewData;

/// All data for whether the renderer needs some kind of update, shared between threads.
#[derive(Debug)]
pub(super) struct StaleData {
    /// Components to update
    needs_update: Mutex<SmallVec<[VComponentPath; 2]>>,
    /// Views to invalidate from other threads (otherwise they're invalidated instantly)
    needs_invalidate_views: Mutex<SmallVec<[NodeId; 2]>>,
    /// Whether we need to rerender.
    /// This should always be set when `needs_update` is modified,
    /// but the invariant is maintained in `Renderer` and not here.
    needs_rerender: NotifyFlag
}

/// Error returned from trying to use `StaleData` because of thread poisoning.
/// Not really handled because it's not really expected.
// As of now these could be separate structs, since no function returns both
#[derive(Debug)]
pub(super) enum StaleDataError {
    NeedsUpdatePoison,
    NeedsInvalidateViewsPoison
}

pub(super) type StaleDataResult<T> = Result<T, NeedsUpdatePoisonError>;

/// Allows you to set that the renderer needs to re-render (no specific components need to update)
/// even from another thread.
#[derive(Debug)]
pub struct NeedsRerenderFlag(Weak<StaleData>);

/// Allows you to set that a specific component (by path) needs to be updated even from another thread.
#[derive(Debug)]
pub struct NeedsUpdateFlag {
    stale_data: Weak<StaleData>,
    path: VComponentPath,
    view_id: NodeId
}

impl NeedsRerenderFlag {
    pub(super) fn from(stale_data: &Arc<StaleData>) -> NeedsRerenderFlag {
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
    pub(super) fn from(stale_data: &Arc<StaleData>, path: VComponentPath, view_id: NodeId) -> NeedsUpdateFlagt {
        NeedsUpdateFlag {
            stale_data: Arc::downgrade(stale_data),
            path,
            view_id
        }
    }

    /// Set this component needs update
    pub fn set(&self) -> StaleDataResult<()> {
        if let Some(stale_data) = self.stale_data.upgrade() {
            stale_data.queue_path_for_update(self.path.clone())?;
            stale_data.queue_view_for_invalidate(self.view_id)?;
            stale_data.needs_rerender.set();
        }
        Ok(())
    }
}

impl StaleData {
    pub(super) fn new() -> Self {
        StaleData {
            needs_update: Mutex::new(SmallVec::new()),
            needs_invalidate_views: Mutex::new(SmallVec::new()),
            needs_rerender: NotifyFlag::new()
        }
    }

    fn needs_update_lock(&self) -> StaleDataResult<MutexGuard<'_, Vec<VComponentPath>>> {
        self.needs_update.lock().map_err(|| StaleDataError::NeedsUpdatePoison)
    }

    fn needs_invalidate_views_lock(&self) -> StaleDataResult<MutexGuard<'_, Vec<NodeId>>> {
        self.needs_invalidate_views.lock().map_err(|| StaleDataError::NeedsInvalidateViewsPoison)
    }

    /// Also should set `needs_rerender` since that is implied by needing to update;
    /// however this function doesn't, `Renderer` which calls this does.
    pub(super) fn queue_path_for_update(&self, path: VComponentPath) -> StaleDataResult<()> {
        self.needs_update_lock()?.push(path);
        Ok(())
    }

    /// Update all components and clear update queue.
    pub(super) fn apply_updates<ViewData: VViewData>(&self, root_component: &mut Box<VComponent<ViewData>>) -> StaleDataResult<()> {
        let mut local_lock = self.needs_update_lock()?;
        for path in local_lock.drain(..) {
            // Component may no longer exist
            if let Some(child_component) = root_component.down_path_mut(&path) {
                child_component.update();
            }
        }
        Ok(())
    }

    /// Call the given function to invalidate all views and clear the invalidate views queue.
    pub(super) fn invalidate_views<ViewData: VViewData>(&self, invalidate_view_fn: impl Fn(NodeId)) -> StaleDataResult<()> {
        let mut local_lock = self.needs_invalidate_views_lock()?;
        for view_id in local_lock.drain(..) {
            invalidate_view_fn(view_id);
        }
        Ok(())
    }

    /// Whether there are queued updates
    pub(super) fn needs_updates(&self) -> bool {
        self.needs_update_lock()
            .map(|needs_update| !needs_update.is_empty())
            .unwrap_or(false)
    }

    /// Whether there are queued views which need to be invalidated (removed from render cache)
    pub(super) fn needs_invalidate_views(&self) -> bool {
        self.needs_invalidate_views_lock()
            .map(|needs_invalidate_views| !needs_invalidate_views.is_empty())
            .unwrap_or(false)
    }

    /// Access the needs_rerender flag to get, set or clear.
    #[allow(clippy::needless_lifetimes)]
    pub(super) fn needs_rerender<'a>(&'a self) -> &'a NotifyFlag {
        &self.needs_rerender
    }
}
