//! Logs component updates and state changes. Part of `Renderer`.

use std::io;
use std::marker::PhantomData;
use crate::core::logging::common::{GenericLogger, LogStart};
use crate::core::view::view::VViewData;
use crate::core::component::update_details::UpdateStack;
#[cfg(feature = "logging")]
use serde::{Serialize, Deserialize};
use crate::core::component::path::VComponentPath;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateLogEntry {
    Update(VComponentPath, UpdateStack),
}

pub struct UpdateLogger<ViewData: VViewData> {
    logger: GenericLogger<UpdateLogEntry>,
    phantom: PhantomData<ViewData>
}

impl <ViewData: VViewData> UpdateLogger<ViewData> {
    pub(in crate::core) fn try_new(args: &LogStart) -> io::Result<Self> {
        Ok(UpdateLogger {
            logger: GenericLogger::new(args, "updates")?,
            phantom: PhantomData
        })
    }

    fn log(&mut self, entry: UpdateLogEntry) {
        self.logger.log(entry)
    }

    pub(in crate::core) fn log_update(&mut self, path: VComponentPath, update_stack: UpdateStack) {
        self.log(UpdateLogEntry::Update(path, update_stack))
    }
}