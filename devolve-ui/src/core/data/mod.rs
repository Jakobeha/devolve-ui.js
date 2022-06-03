pub mod data;
#[cfg(feature = "obs-ref")]
pub mod obs_ref;

pub use devolve_ui_derive::Data;
#[cfg(feature = "obs-ref")]
pub use devolve_ui_derive::ObsRefable;