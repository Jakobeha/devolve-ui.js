#![feature(arbitrary_self_types)]
#![feature(decl_macro)]

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

//! derive macros for devolve-ui datatypes, from druid.

#![deny(clippy::trivially_copy_pass_by_ref)]

extern crate proc_macro;

mod attr;
mod data;
mod obs_ref;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Generates implementations of the `Data` trait.
///
/// This macro supports a `data` field attribute with the following arguments:
///
/// - `#[data(ignore)]` makes the generated `Data::same` function skip comparing this field.
/// - `#[data(same_fn="foo")]` uses the function `foo` for comparing this field. `foo` should
///    be the name of a function with signature `fn(&T, &T) -> bool`, where `T` is the type of
///    the field.
/// - `#[data(eq)]` is shorthand for `#[data(same_fn = "PartialEq::eq")]`
///
/// # Example
///
/// ```rust
/// use devolve_ui_derive::Data;
///
/// #[derive(Clone, Data)]
/// struct State {
///     number: f64,
///     // `Vec` doesn't implement `Data`, so we need to either ignore it or supply a `same_fn`.
///     #[data(eq)]
///     // same as #[data(same_fn="PartialEq::eq")]
///     indices: Vec<usize>,
///     // This is just some sort of cache; it isn't important for sameness comparison.
///     #[data(ignore)]
///     cached_indices: Vec<usize>,
/// }
/// ```
#[proc_macro_derive(Data, attributes(data))]
pub fn derive_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    data::derive_data_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Generates implementations of the `ObsRefable` trait.
///
/// This macro supports a `obs_ref` field attribute with the following arguments:
///
/// - `#[obs_ref(ignore)]` makes the generated `ObsRef` not support this field.
/// - `#[obs_ref(derive)]` is necessary for not-ignored fields whose types don't implement 'ObsRefable'.
///
/// # Example
///
/// ```rust
/// use devolve_ui_derive::ObsRefable;
///
/// #[derive(Clone, ObsRefable)]
/// struct State {
///     number: f64,
///     indices: Vec<usize>,
///     #[obs_ref(derive)]
///     fancy_indices: FancyVec<usize>,
///     #[obs_ref(ignore)]
///     id_which_should_be_readonly: usize
/// }
/// ```
#[proc_macro_derive(ObsRefable, attributes(obs_ref))]
pub fn derive_obs_ref(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    obs_ref::derive_obs_ref_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}