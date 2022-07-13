#![feature(arbitrary_self_types)]
#![feature(decl_macro)]
#![feature(downcast_unchecked)]
#![feature(generic_associated_types)]
#![feature(is_some_with)]
#![feature(const_trait_impl)]
#![feature(const_for)]
#![feature(const_try)]
#![feature(const_float_classify)]
#![feature(const_mut_refs)]
#![feature(const_intoiterator_identity)]
#![feature(const_convert)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(negative_impls)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(dispatch_from_dyn)]
#![feature(map_try_insert)]
#![feature(cfg_version)]
#![feature(maybe_uninit_write_slice)]
#![feature(iter_collect_into)]
#![cfg_attr(not(version("1.63")), feature(explicit_generic_args_with_impl_trait))]
#![cfg_attr(feature = "backtrace", feature(backtrace))]

pub mod core;
#[cfg(feature = "wasm")]
pub mod wasm;
pub mod engines;
pub mod view_data;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
