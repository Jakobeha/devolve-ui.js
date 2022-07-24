#![feature(cfg_version)]
#![feature(is_some_with)]
#![feature(map_try_insert)]
#![feature(generic_associated_types)]
#![feature(const_trait_impl)]
#![feature(const_try)]
#![feature(const_float_classify)]
#![feature(const_convert)]
#![feature(const_mut_refs)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(decl_macro)]
#![feature(negative_impls)]
#![feature(downcast_unchecked)]
#![cfg_attr(not(version("1.63")), feature(explicit_generic_args_with_impl_trait))]
#![cfg_attr(feature = "backtrace", feature(backtrace))]

pub mod component;
pub mod data;
pub mod hooks;
#[cfg(feature = "logging")]
pub mod logging;
pub mod renderer;
pub mod view;
pub mod misc;
