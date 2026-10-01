//! Module: girra/src/types/composite.rs
//!
//! Composite, Reference, Container, and Heap types in Rust AST.

use super::Type;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<StructField>,
}

impl StructType {
    pub fn to_token_stream(&self) -> TokenStream {
        let ident = Ident::new(&self.name, proc_macro2::Span::call_site());
        quote! { #ident }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TupleType {
    pub elements: Vec<Type>,
}

impl TupleType {
    pub fn to_token_stream(&self) -> TokenStream {
        let elems: Vec<TokenStream> = self.elements.iter().map(|e| e.to_token_stream()).collect();
        quote! { ( #(#elems),* ) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayType {
    pub element_type: Box<Type>,
    pub size: usize,
}

impl ArrayType {
    pub fn to_token_stream(&self) -> TokenStream {
        let elem = self.element_type.to_token_stream();
        let size = self.size;
        quote! { [#elem; #size] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceType {
    pub element_type: Box<Type>,
    pub mutable: bool,
    pub lifetime_id: Option<usize>,
}

impl ReferenceType {
    pub fn to_token_stream(&self) -> TokenStream {
        let elem = self.element_type.to_token_stream();
        let lifetime = self.lifetime_id.map(|id| {
            let lt_ident = Ident::new(&format!("'a{}", id), proc_macro2::Span::call_site());
            quote! { #lt_ident }
        });

        if self.mutable {
            if let Some(lt) = lifetime {
                quote! { &#lt mut #elem }
            } else {
                quote! { &mut #elem }
            }
        } else if let Some(lt) = lifetime {
            quote! { &#lt #elem }
        } else {
            quote! { &#elem }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HeapType {
    Vec(Box<Type>),
    Box(Box<Type>),
}

impl HeapType {
    pub fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::Vec(inner) => {
                let elem = inner.to_token_stream();
                quote! { Vec<#elem> }
            }
            Self::Box(inner) => {
                let elem = inner.to_token_stream();
                quote! { Box<#elem> }
            }
        }
    }
}
