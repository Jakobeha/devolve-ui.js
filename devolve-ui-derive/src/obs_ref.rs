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
use crate::attr::{ObsRefAttr, Field, FieldKind, Fields};

use quote::{quote, quote_spanned};
use syn;
use syn::{spanned::Spanned, Data, DataEnum, DataStruct};

pub(crate) fn derive_obs_ref_impl(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
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
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let ident = &input.ident;
    let (_, ty_generics, where_clause) = &input.generics.split_for_impl();
    let impl_generics_root = generics_bounds(&input.generics, &quote!(#ident #ty_generics));
    let impl_generics_child = generics_bounds(&input.generics, &quote!(Root));
    let ty_generics2 = ty_generics2(&input.generics);

    let all_fields = Fields::<ObsRefAttr>::parse_ast(&s.fields)?;
    let fields = || {
        all_fields
            .iter()
            .filter(|f| f.attrs != ObsRefAttr::Ignore)
    };

    let field_ids: Vec<_> = fields().map(Field::coerced_ident).collect();
    let types: Vec<_> = fields().map(|field| &field.ty).collect();
    let obs_ref_fields = quote!( #( #field_ids: <#types as ::devolve_ui::core::data::obs_ref::ObsRefableChild<#ident>>::ObsRefImpl ),* );
    let obs_ref_base_field = ident_from_str("__base");

    let obs_ref_root = ident_from_str(&format!("ObsRefRootFor{}", ident));
    let obs_ref_child = ident_from_str(&format!("ObsRefChildFor{}", ident));

    let res = quote! {
        pub struct #obs_ref_root <#impl_generics_root> #where_clause {
            #obs_ref_base_field: ObsRefRootBase<#ident #ty_generics>,
            #( pub #field_ids: <#types as ::devolve_ui::core::data::obs_ref::ObsRefableChild<#ident #ty_generics>>::ObsRefImpl ),*
        }

        pub struct #obs_ref_child <Root, #impl_generics_child> #where_clause {
            #obs_ref_base_field: ObsRefChildBase<#ident #ty_generics>,
            #( pub #field_ids: <#types as ::devolve_ui::core::data::obs_ref::ObsRefableChild<Root>>::ObsRefImpl ),*
        }

        impl <#impl_generics_root> ::devolve_ui::core::data::obs_ref::ObsRefableRoot for #ident #ty_generics #where_clause {
            type ObsRefImpl = #obs_ref_root #ty_generics #where_clause;

            fn to_obs_ref(self: Self) -> Self::ObsRefImpl {
                unsafe {
                    let mut base = ::devolve_ui::core::data::obs_ref::ObsRefRootBase::new(self);
                    #obs_ref_root {
                        #obs_ref_base_field: base,
                        #( #field_ids: base.root_value.#field_ids.to_obs_ref("", stringify!(#field_ids), base.downgrade()) ),*
                    }
                }
            }
        }

        impl <Root, #impl_generics_child> ::devolve_ui::core::data::obs_ref::ObsRefableChild<Root> for #ident #ty_generics #where_clause {
            type ObsRefImpl = #obs_ref_child<Root #ty_generics2> #where_clause;

            unsafe fn _to_obs_ref(self: *mut Self, path: String, root: Weak<ObsRefRootBase<Root>>) -> Self::ObsRefImpl {
                let mut base = ::devolve_ui::core::data::obs_ref::ObsRefChildBase::new(self, path, root.clone());
                #obs_ref_child {
                    #obs_ref_base_field: base,
                    #( #field_ids: base.child_value.#field_ids.to_obs_ref(path, stringify!(#field_ids), root.clone()) ),*
                }
            }
        }

        impl <#impl_generics_root> ::devolve_ui::core::data::obs_ref::ObsRef<#ident #ty_generics, #ident #ty_generics> for #obs_ref_root #ty_generics #where_clause {
            fn i(&self) -> &#ident #ty_generics {
                self.#obs_ref_base_field.i()
            }

            fn m(&self) -> ObsDeref<#ident ty_generics, #ident #ty_generics> {
                self.#obs_ref_base_field.m()
            }

            fn after_mutate(&self, observer: ::devolve_ui::core::data::obs_ref::Observer<Root>) {
                self.#obs_ref_base_field.after_mutate(observer)
            }

            fn base(&self) -> Weak<ObsRefRootBase<Root>> {
                self.#obs_ref_base_field.base()
            }
        }

        impl <Root, #impl_generics_child> ::devolve_ui::core::data::obs_ref::ObsRef<Root, #ident #ty_generics> for #obs_ref_child<Root, #ty_generics2> #where_clause {
            fn i(&self) -> &#ident #ty_generics {
                self.#obs_ref_base_field.i()
            }

            fn m(&self) -> ObsDeref<#ident ty_generics, #ident #ty_generics> {
                self.#obs_ref_base_field.m()
            }

            fn after_mutate(&self, observer: ::devolve_ui::core::data::obs_ref::Observer<Root>) {
                self.#obs_ref_base_field.after_mutate(observer)
            }

            fn base(&self) -> Weak<ObsRefRootBase<Root>> {
                self.#obs_ref_base_field.base()
            }
        }
    };

    Ok(res)
}

fn ident_from_str(s: &str) -> proc_macro2::Ident {
    proc_macro2::Ident::new(s, proc_macro2::Span::call_site())
}

fn generics_bounds(generics: &syn::Generics, root_ident: &TokenStream) -> proc_macro2::TokenStream {
    let res = generics.params.iter().map(|gp| {
        use syn::GenericParam::*;
        match gp {
            syn::GenericParam::Type(ty) => {
                let ident = &ty.ident;
                let bounds = &ty.bounds;
                if bounds.is_empty() {
                    quote_spanned!(ty.span()=> #ident : ::devolve_ui::core::obs_ref::ObsRefableChild<#root_ident>)
                } else {
                    quote_spanned!(ty.span()=> #ident : #bounds + ::devolve_ui::core::obs_ref::ObsRefableChild<#root_ident>)
                }
            }
            syn::GenericParam::Lifetime(lf) => quote!(#lf),
            syn::GenericParam::Const(cst) => quote!(#cst),
        }
    });

    quote!( #( #res, )* )
}

fn ty_generics2(generics: &syn::Generics) -> proc_macro2::TokenStream {
    let params = generics.params.iter();

    quote!( #( ,#params )* )
}
