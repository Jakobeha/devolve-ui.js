
use proc_macro::TokenStream;
use syn::{ExprStruct, parse_macro_input};
use quote::quote;
use itertools::Itertools;

#[proc_macro]
pub fn dedup_struct_fields(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as ExprStruct);
    let fields = expr.fields.into_iter().dedup_by(|a, b| a.member == b.member);
    let expr = ExprStruct {
        attrs: expr.attrs,
        path: expr.path,
        brace_token: expr.brace_token,
        fields: fields.collect(),
        dot2_token: expr.dot2_token,
        rest: expr.rest
    };
    TokenStream::from(quote!(#expr))
}