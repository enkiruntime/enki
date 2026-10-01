//! Module: girra/src/types/lifetime.rs
//!
//! Lifetime tracking and generic parameter formatting for Rust references.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::Ident;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LifetimeId(pub usize);

impl LifetimeId {
    pub fn to_ident(self) -> Ident {
        Ident::new(&format!("'a{}", self.0), proc_macro2::Span::call_site())
    }

    pub fn to_token_stream(self) -> TokenStream {
        let ident = self.to_ident();
        quote! { #ident }
    }
}

pub fn format_generics(lifetimes: &BTreeSet<LifetimeId>) -> TokenStream {
    if lifetimes.is_empty() {
        quote! {}
    } else {
        let lt_tokens: Vec<TokenStream> = lifetimes.iter().map(|lt| lt.to_token_stream()).collect();
        quote! { < #(#lt_tokens),* > }
    }
}
