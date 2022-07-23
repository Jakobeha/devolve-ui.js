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
#![feature(cell_update)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(into_future)]
#![feature(new_uninit)]
#![feature(local_key_cell_methods)]
#![cfg_attr(not(version("1.63")), feature(explicit_generic_args_with_impl_trait))]
#![cfg_attr(feature = "backtrace", feature(backtrace))]

pub mod core;
// TODO: Move everything outside of core to their own packages and move crate::core to crate.
//   This stuff isn't features
#[cfg(feature = "wasm")]
pub mod wasm;
pub mod prompt;
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
