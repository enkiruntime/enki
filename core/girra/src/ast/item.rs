use super::stmt::Stmt;
use crate::types::{StructField, Type, format_generics};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub is_repr_c: bool,
}

impl StructDef {
    pub fn to_token_stream(&self) -> TokenStream {
        let name_ident = Ident::new(&self.name, proc_macro2::Span::call_site());
        let repr_attr = if self.is_repr_c {
            quote! { #[repr(C)] }
        } else {
            quote! {}
        };

        let mut lifetimes = std::collections::BTreeSet::new();
        for f in &self.fields {
            lifetimes.extend(f.ty.lifetime_parameters());
        }
        let generics_t = format_generics(&lifetimes);

        let fields_t: Vec<TokenStream> = self
            .fields
            .iter()
            .map(|f| {
                let f_name = Ident::new(&f.name, proc_macro2::Span::call_site());
                let f_ty = f.ty.to_token_stream();
                quote! { pub #f_name: #f_ty }
            })
            .collect();

        quote! {
            #repr_attr
            #[derive(Copy, Clone, Debug, PartialEq, Default)]
            pub struct #name_ident #generics_t {
                #(#fields_t),*
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDef {
    pub name: String,
    pub args: Vec<(String, Type)>,
    pub return_type: Type,
    pub body: Vec<Stmt>,
}

impl FnDef {
    pub fn to_token_stream(&self) -> TokenStream {
        let name_ident = Ident::new(&self.name, proc_macro2::Span::call_site());
        let ret_t = self.return_type.to_token_stream();

        let args_t: Vec<TokenStream> = self
            .args
            .iter()
            .map(|(arg_name, arg_ty)| {
                let arg_ident = Ident::new(arg_name, proc_macro2::Span::call_site());
                let ty_t = arg_ty.to_token_stream();
                quote! { #arg_ident: #ty_t }
            })
            .collect();

        let body_t: Vec<TokenStream> = self.body.iter().map(|s| s.to_token_stream()).collect();

        quote! {
            pub fn #name_ident(#(#args_t),*) -> #ret_t {
                #(#body_t)*
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamDef {
    pub name: String,
    pub space_param: String,
    pub args: Vec<(String, Type)>,
    pub body: Vec<Stmt>,
}

impl NamDef {
    pub fn to_token_stream(&self) -> TokenStream {
        let name_ident = Ident::new(&self.name, proc_macro2::Span::call_site());
        let space_ident = Ident::new(&self.space_param, proc_macro2::Span::call_site());

        let args_t: Vec<TokenStream> = self
            .args
            .iter()
            .map(|(arg_name, arg_ty)| {
                let arg_ident = Ident::new(arg_name, proc_macro2::Span::call_site());
                let ty_t = arg_ty.to_token_stream();
                quote! { #arg_ident: #ty_t }
            })
            .collect();

        let body_t: Vec<TokenStream> = self.body.iter().map(|s| s.to_token_stream()).collect();

        quote! {
            #[nam]
            pub fn #name_ident(#space_ident: &Space, #(#args_t),*) {
                #(#body_t)*
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAliasDef {
    pub name: String,
    pub target: Type,
}

impl TypeAliasDef {
    pub fn to_token_stream(&self) -> TokenStream {
        let name_ident = Ident::new(&self.name, proc_macro2::Span::call_site());
        let target_t = self.target.to_token_stream();
        quote! {
            pub type #name_ident = #target_t;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub seed: u64,
    pub structs: Vec<StructDef>,
    pub type_aliases: Vec<TypeAliasDef>,
    pub functions: Vec<FnDef>,
    pub nam_kernel: Option<NamDef>,
}

impl Program {
    pub fn to_rust_source(&self) -> String {
        let structs_t: Vec<TokenStream> =
            self.structs.iter().map(|s| s.to_token_stream()).collect();
        let aliases_t: Vec<TokenStream> = self
            .type_aliases
            .iter()
            .map(|a| a.to_token_stream())
            .collect();
        let fns_t: Vec<TokenStream> = self.functions.iter().map(|f| f.to_token_stream()).collect();
        let nam_t = self.nam_kernel.as_ref().map(|k| k.to_token_stream());

        let program_tokens = quote! {
            #![allow(warnings, unused, unconditional_panic, dead_code)]
            use enki::*;

            #(#structs_t)*
            #(#aliases_t)*
            #(#fns_t)*
            #nam_t
        };

        if let Ok(file) = syn::parse2::<syn::File>(program_tokens.clone()) {
            prettyplease::unparse(&file)
        } else {
            program_tokens.to_string()
        }
    }
}
