//! Logs component updates and state changes. Part of `Renderer`.

use crate::core::logging::common::GenericLogger;
use crate::core::view::view::VViewData;
use crate::core::component::update_details::UpdateFrame;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateLogEntry {
    Update(UpdateFrame),
}

pub struct UpdateLogger<ViewData: VViewData> {
    logger: GenericLogger<UpdateLogEntry>
}

impl <ViewData: VViewData> UpdateLogger<ViewData> {
    pub(in crate::core) fn log(&mut self, entry: UpdateLogEntry) {
        self.logger.log(entry)
    }
}