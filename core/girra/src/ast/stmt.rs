use super::expr::Expr;
use crate::types::Type;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let {
        is_mutable: bool,
        name: String,
        ty: Option<Type>,
        init: Expr,
    },
    Assign {
        lhs: Expr,
        rhs: Expr,
    },
    Expr {
        expr: Expr,
        has_semicolon: bool,
    },
    Return(Option<Expr>),
    Break,
    Const {
        name: String,
        ty: Type,
        value: Expr,
    },
}

impl Stmt {
    pub fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::Let {
                is_mutable,
                name,
                ty,
                init,
            } => {
                let ident = Ident::new(name, proc_macro2::Span::call_site());
                let mut_t = if *is_mutable {
                    quote! { mut }
                } else {
                    quote! {}
                };
                let ty_t = ty.as_ref().map(|t| {
                    let ts = t.to_token_stream();
                    quote! { : #ts }
                });
                let init_t = init.to_token_stream();
                quote! { let #mut_t #ident #ty_t = #init_t; }
            }
            Self::Assign { lhs, rhs } => {
                let lhs_t = lhs.to_token_stream();
                let rhs_t = rhs.to_token_stream();
                quote! { #lhs_t = #rhs_t; }
            }
            Self::Expr {
                expr,
                has_semicolon,
            } => {
                let expr_t = expr.to_token_stream();
                if *has_semicolon {
                    quote! { #expr_t; }
                } else {
                    quote! { #expr_t }
                }
            }
            Self::Return(opt_expr) => {
                if let Some(e) = opt_expr {
                    let e_t = e.to_token_stream();
                    quote! { return #e_t; }
                } else {
                    quote! { return; }
                }
            }
            Self::Break => quote! { break; },
            Self::Const { name, ty, value } => {
                let ident = Ident::new(name, proc_macro2::Span::call_site());
                let ty_t = ty.to_token_stream();
                let val_t = value.to_token_stream();
                quote! { const #ident: #ty_t = #val_t; }
            }
        }
    }
}
