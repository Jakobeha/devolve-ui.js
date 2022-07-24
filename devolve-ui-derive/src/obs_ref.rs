// Copyright 2019 The Druid Authors.
// - Modified 2022 jakobeha
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The implementation for #[derive(ObsRef)]

use proc_macro2::TokenStream;
use crate::attr::{ObsRefAttr, Field, Fields};

use quote::{quote, quote_spanned};
use syn;
use syn::{spanned::Spanned, Data, DataStruct};

pub(crate) fn derive_obs_ref_impl(
    input: syn::DeriveInput,
) -> Result<TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(s) => derive_struct(&input, s),
        Data::Enum(e) => Err(syn::Error::new(
            e.enum_token.span(),
            "ObsRef implementations cannot be derived from enums",
        )),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "Data implementations cannot be derived from unions",
        )),
    }
}

fn derive_struct(
    input: &syn::DeriveInput,
    s: &DataStruct,
) -> Result<TokenStream, syn::Error> {
    let vis = &input.vis;
    let ident = &input.ident;
    let (_, ty_generics, where_clause) = &input.generics.split_for_impl();
    let impl_generics_root = generics_bounds(&input.generics, &quote!(#ident #ty_generics));
    let impl_generics_child = generics_bounds(&input.generics, &quote!(Root));

    let all_fields = Fields::<ObsRefAttr>::parse_ast(&s.fields)?;
    let fields = || {
        all_fields
            .iter()
            .filter(|f| f.attrs != ObsRefAttr::Ignore)
    };

    let field_ids: Vec<_> = fields().map(Field::coerced_ident).collect();
    let types: Vec<_> = fields().map(|field| &field.ty).collect();
    let obs_ref_base_field = ident_from_str("__base");

    let obs_ref_root = ident_from_str(&format!("ObsRefRootFor{}", ident));
    let obs_ref_child = ident_from_str(&format!("ObsRefChildFor{}", ident));

    let ty_generics_and = ty_generics_and(&input.generics);

    let res = quote! {
        #[automatically_derived]
        #vis struct #obs_ref_root <#impl_generics_root S: ::devolve_ui::data::obs_ref::st::SubCtx> #where_clause {
            #obs_ref_base_field: ::std::rc::Rc<::devolve_ui::data::obs_ref::st::ObsRefRootBase<#ident #ty_generics, S>>,
            #( pub #field_ids: <#types as ::devolve_ui::data::obs_ref::st::ObsRefableChild<#ident #ty_generics, S>>::ObsRefImpl ),*
        }

        #[automatically_derived]
        #vis struct #obs_ref_child <#impl_generics_child Root, S: ::devolve_ui::data::obs_ref::st::SubCtx> #where_clause {
            #obs_ref_base_field: ::devolve_ui::data::obs_ref::st::ObsRefChildBase<Root, #ident #ty_generics, S>,
            #( pub #field_ids: <#types as ::devolve_ui::data::obs_ref::st::ObsRefableChild<Root, S>>::ObsRefImpl ),*
        }

        #[automatically_derived]
        impl <#impl_generics_root S: ::devolve_ui::data::obs_ref::st::SubCtx> ::devolve_ui::data::obs_ref::st::ObsRefableRoot<S> for #ident #ty_generics #where_clause {
            type ObsRefImpl = #obs_ref_root<#ty_generics_and S> #where_clause;

            fn into_obs_ref(self: Self) -> Self::ObsRefImpl {
                use ::devolve_ui::data::obs_ref::st::ObsRefableChild;
                unsafe {
                    let mut base = ::devolve_ui::data::obs_ref::st::ObsRefRootBase::new(self);
                    #obs_ref_root {
                        #( #field_ids: base.root_value().#field_ids.as_obs_ref_child(
                            &[],
                            base.pending(),
                            "",
                            stringify!(#field_ids),
                            base.clone()
                        ), )*
                        #obs_ref_base_field: base
                    }
                }
            }
        }

        #[automatically_derived]
        impl <#impl_generics_child Root, S: ::devolve_ui::data::obs_ref::st::SubCtx> ::devolve_ui::data::obs_ref::st::ObsRefableChild<Root, S> for #ident #ty_generics #where_clause {
            type ObsRefImpl = #obs_ref_child<#ty_generics_and Root, S> #where_clause;

            unsafe fn _as_obs_ref_child(
                this: *mut Self,
                ancestors_pending: &[::std::rc::Weak<::devolve_ui::data::obs_ref::st::ObsRefPending<S>>],
                parent_pending: &::std::rc::Rc<::devolve_ui::data::obs_ref::st::ObsRefPending<S>>,
                path: String,
                root: ::std::rc::Rc<::devolve_ui::data::obs_ref::st::ObsRefRootBase<Root, S>>
            ) -> Self::ObsRefImpl {
                use ::devolve_ui::data::obs_ref::st::ObsRefableChild;
                let mut base = ::devolve_ui::data::obs_ref::st::ObsRefChildBase::new(this, ancestors_pending, parent_pending, path, root.clone());
                #obs_ref_child {
                    #( #field_ids: base.child_value().#field_ids.as_obs_ref_child(
                        base.parents_pending(),
                        base.pending(),
                        base.path(),
                        stringify!(#field_ids),
                        root.clone()
                    ), )*
                    #obs_ref_base_field: base
                }
            }
        }

        #[automatically_derived]
        impl <#impl_generics_root S: ::devolve_ui::data::obs_ref::st::SubCtx> ::devolve_ui::data::obs_ref::st::ObsRef<#ident #ty_generics, #ident #ty_generics, S> for #obs_ref_root<#ty_generics_and S> #where_clause {
            fn i(&self, s: S::Input<'_>) -> &#ident #ty_generics {
                self.#obs_ref_base_field.i(s)
            }

            fn m(&mut self, s: S::Input<'_>) -> ::devolve_ui::data::obs_ref::st::ObsDeref<#ident #ty_generics, #ident #ty_generics, S> {
                self.#obs_ref_base_field.m(s)
            }

            fn after_mutate(&self, observer: ::devolve_ui::data::obs_ref::st::Observer<#ident #ty_generics, S>) {
                self.#obs_ref_base_field.after_mutate(observer)
            }

            fn base(&self) -> &::std::rc::Rc<::devolve_ui::data::obs_ref::st::ObsRefRootBase<#ident #ty_generics, S>> {
                self.#obs_ref_base_field.base()
            }
        }

        #[automatically_derived]
        impl <#impl_generics_child Root, S: ::devolve_ui::data::obs_ref::st::SubCtx> ::devolve_ui::data::obs_ref::st::ObsRef<Root, #ident #ty_generics, S> for #obs_ref_child<#ty_generics_and Root, S> #where_clause {
            fn i(&self, s: S::Input<'_>) -> &#ident #ty_generics {
                self.#obs_ref_base_field.i(s)
            }

            fn m(&mut self, s: S::Input<'_>) -> ::devolve_ui::data::obs_ref::st::ObsDeref<Root, #ident #ty_generics, S> {
                self.#obs_ref_base_field.m(s)
            }

            fn after_mutate(&self, observer: ::devolve_ui::data::obs_ref::st::Observer<Root, S>) {
                self.#obs_ref_base_field.after_mutate(observer)
            }

            fn base(&self) -> &::std::rc::Rc<::devolve_ui::data::obs_ref::st::ObsRefRootBase<Root, S>> {
                self.#obs_ref_base_field.base()
            }
        }
    };
    Ok(res)
}

fn ident_from_str(s: &str) -> proc_macro2::Ident {
    proc_macro2::Ident::new(s, proc_macro2::Span::call_site())
}

fn generics_bounds(generics: &syn::Generics, root_ident: &TokenStream) -> TokenStream {
    let res = generics.params.iter().map(|gp| {
        match gp {
            syn::GenericParam::Type(ty) => {
                let ident = &ty.ident;
                let bounds = &ty.bounds;
                if bounds.is_empty() {
                    quote_spanned!(ty.span()=> #ident : ::devolve_ui::obs_ref::ObsRefableChild<#root_ident, S>)
                } else {
                    quote_spanned!(ty.span()=> #ident : #bounds + ::devolve_ui::obs_ref::ObsRefableChild<#root_ident, S>)
                }
            }
            syn::GenericParam::Lifetime(lf) => quote!(#lf),
            syn::GenericParam::Const(cst) => quote!(#cst),
        }
    });

    quote!( #( #res, )* )
}

fn ty_generics_and(generics: &syn::Generics) -> TokenStream {
    let params = generics.params.iter();

    quote!( #( #params, )* )
}
