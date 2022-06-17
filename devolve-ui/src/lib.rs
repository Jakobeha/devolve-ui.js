#![feature(arbitrary_self_types)]
#![feature(decl_macro)]
#![feature(explicit_generic_args_with_impl_trait)]
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
#![cfg_attr(feature = "backtrace", feature(backtrace))]

pub mod core;
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
