#![feature(arbitrary_self_types)]
// #![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(is_some_with)]
#![feature(explicit_generic_args_with_impl_trait)]

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
