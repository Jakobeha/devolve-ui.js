//! Logs component updates and state changes. Part of `Renderer`.

use std::io;
use std::marker::PhantomData;
use crate::logging::common::{GenericLogger, LogStart};
use crate::view::view::VViewData;
use crate::component::update_details::UpdateStack;
#[cfg(feature = "logging")]
use serde::{Serialize, Deserialize};
use crate::component::path::VComponentPath;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateLogEntry {
    Update(VComponentPath, UpdateStack),
}

pub struct UpdateLogger<ViewData: VViewData> {
    logger: GenericLogger<UpdateLogEntry>,
    phantom: PhantomData<ViewData>
}

impl <ViewData: VViewData> UpdateLogger<ViewData> {
    pub(crate) fn try_new(args: &LogStart) -> io::Result<Self> {
        Ok(UpdateLogger {
            logger: GenericLogger::new(args, "updates")?,
            phantom: PhantomData
        })
    }

    fn log(&mut self, entry: UpdateLogEntry) {
        self.logger.log(entry)
    }

    pub(crate) fn log_update(&mut self, path: VComponentPath, update_stack: UpdateStack) {
        self.log(UpdateLogEntry::Update(path, update_stack))
    }
}