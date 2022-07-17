//! Random utility code which aren't really `devolve-ui` specific but needed by `devolve-ui`.

pub(crate) mod assert_variance;
pub mod float_ord;
pub mod ident;
pub mod io_write_2_fmt_write;
pub mod is_thread_safe;
pub mod fmt_mode;
pub(crate) mod map_lock_result;
pub(crate) mod map_split_n;
pub mod notify_flag;
pub mod option_f32;
pub mod partial_default;
pub mod hash_map_ref_stack;
pub mod ref_stack;
pub mod shorthand;
pub mod slice_split3;

// TODO move
pub(crate) mod frozen_vec;
pub(crate) mod stable_deref2;