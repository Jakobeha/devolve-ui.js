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

//! parsing #[devolve-ui(attributes)]

use proc_macro2::{Ident, Literal, Span, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::{Error, ExprPath, Meta, NestedMeta};

use quote::{quote, quote_spanned};

/// The fields for a struct or an enum variant.
#[derive(Debug)]
pub struct Fields<Attrs> {
    pub kind: FieldKind,
    fields: Vec<Field<Attrs>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    Named,
    // this also covers Unit; we determine 'unit-ness' based on the number
    // of fields.
    Unnamed,
}

#[derive(Debug)]
pub enum FieldIdent {
    Named(String),
    Unnamed(usize),
}

pub trait Attrs: Default {
    const BASE_PATH: &'static str;

    fn add_attr(&mut self, attr: &syn::Attribute) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct Field<Attrs> {
    pub ident: FieldIdent,
    pub ty: syn::Type,

    pub attrs: Attrs,
}

#[derive(Debug, PartialEq)]
pub enum DataAttr {
    Empty,
    Ignore,
    SameFn(ExprPath),
    Eq,
}

#[derive(Debug, PartialEq)]
pub enum ObsRefAttr {
    Empty,
    Ignore,
    Derive
}

impl <A: Attrs> Fields<A> {
    pub fn parse_ast(fields: &syn::Fields) -> Result<Self, Error> {
        let kind = match fields {
            syn::Fields::Named(_) => FieldKind::Named,
            syn::Fields::Unnamed(_) | syn::Fields::Unit => FieldKind::Unnamed,
        };

        let fields = fields
            .iter()
            .enumerate()
            .map(|(i, field)| Field::<A>::parse_ast(field, i))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Fields { kind, fields })
    }
}

impl <Attrs> Fields<Attrs> {
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Field<Attrs>> {
        self.fields.iter()
    }
}

impl <A: Attrs> Field<A> {
    pub fn parse_ast(field: &syn::Field, index: usize) -> Result<Self, Error> {
        let ident = match field.ident.as_ref() {
            Some(ident) => FieldIdent::Named(ident.to_string().trim_start_matches("r#").to_owned()),
            None => FieldIdent::Unnamed(index),
        };

        let ty = field.ty.clone();

        let mut target_attrs = A::default();
        for attr in field.attrs.iter() {
            if attr.path.is_ident(A::BASE_PATH) {
                target_attrs.add_attr(attr)?;
            }
        }
        Ok(Field {
            ident,
            ty,
            attrs: target_attrs,
        })
    }
}

impl Field<DataAttr> {
    /// The tokens to be used as the function for 'same'.
    pub fn same_fn_path_tokens(&self) -> TokenStream {
        match &self.attrs {
            DataAttr::SameFn(f) => quote!(#f),
            DataAttr::Eq => quote!(::cmp::PartialEq::eq),
            // this should not be called for DataAttr::Ignore
            DataAttr::Ignore => quote!(compiler_error!),
            DataAttr::Empty => {
                let span = Span::call_site();
                quote_spanned!(span=> devolve_ui::data::Data::same)
            }
        }
    }
}

impl Default for DataAttr {
    fn default() -> Self {
        DataAttr::Empty
    }
}

impl Attrs for DataAttr {
    const BASE_PATH: &'static str = "data";

    fn add_attr(&mut self, attr: &syn::Attribute) -> Result<(), Error> {
        match attr.parse_meta()? {
            Meta::List(meta) => {
                assert!(
                    meta.nested.len() == 1,
                    "only single data attribute is allowed"
                );
                let nested = meta.nested.first().unwrap();
                match nested {
                    NestedMeta::Meta(Meta::Path(path))
                    if path.is_ident("ignore") =>
                        {
                            *self = DataAttr::Ignore;
                        }
                    NestedMeta::Meta(Meta::NameValue(meta))
                    if meta.path.is_ident("same_fn") =>
                        {
                            let path = parse_lit_into_expr_path(&meta.lit)?;
                            *self = DataAttr::SameFn(path);
                        }
                    NestedMeta::Meta(Meta::Path(path))
                    if path.is_ident("eq") =>
                        {
                            *self = DataAttr::Eq;
                        }
                    other => return Err(Error::new(other.span(), "Unknown attribute")),
                }
            }
            other => {
                return Err(Error::new(
                    other.span(),
                    "Expected attribute list (the form #[data(...)]",
                ));
            }
        }
        Ok(())
    }
}

impl Default for ObsRefAttr {
    fn default() -> Self {
        ObsRefAttr::Empty
    }
}

impl Attrs for ObsRefAttr {
    const BASE_PATH: &'static str = "obs_ref";

    fn add_attr(&mut self, attr: &syn::Attribute) -> Result<(), Error> {
        match attr.parse_meta()? {
            Meta::List(meta) => {
                assert!(
                    meta.nested.len() == 1,
                    "only single data attribute is allowed"
                );
                let nested = meta.nested.first().unwrap();
                match nested {
                    NestedMeta::Meta(Meta::Path(path))
                    if path.is_ident("ignore") =>
                        {
                            *self = ObsRefAttr::Ignore;
                        }
                    NestedMeta::Meta(Meta::Path(path))
                    if path.is_ident("derive") =>
                        {
                            *self = ObsRefAttr::Derive;
                        }
                    other => return Err(Error::new(other.span(), "Unknown attribute")),
                }
            }
            other => {
                return Err(Error::new(
                    other.span(),
                    "Expected attribute list (the form #[obs_ref(...)])",
                ));
            }
        }
        Ok(())
    }
}

impl<Attrs> Field<Attrs> {
    pub fn ident_tokens(&self) -> TokenTree {
        match self.ident {
            FieldIdent::Named(ref s) => Ident::new(s, Span::call_site()).into(),
            FieldIdent::Unnamed(num) => Literal::usize_unsuffixed(num).into(),
        }
    }

    pub fn coerced_ident(&self) -> Ident {
        match self.ident {
            FieldIdent::Named(ref s) => Ident::new(s, Span::call_site()),
            FieldIdent::Unnamed(num) => Ident::new(&format!("_{}", num), Span::call_site()),
        }
    }

    pub fn ident_string(&self) -> String {
        match self.ident {
            FieldIdent::Named(ref s) => s.clone(),
            FieldIdent::Unnamed(num) => num.to_string(),
        }
    }
}

fn parse_lit_into_expr_path(lit: &syn::Lit) -> Result<ExprPath, Error> {
    let string = if let syn::Lit::Str(lit) = lit {
        lit
    } else {
        return Err(Error::new(
            lit.span(),
            "expected str, found... something else",
        ));
    };

    let tokens = syn::parse_str(&string.value())?;
    syn::parse2(tokens)
}