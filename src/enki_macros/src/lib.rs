extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, Type};

#[proc_macro_attribute]
pub fn nam(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_fn = match syn::parse::<ItemFn>(item.clone()) {
        Ok(f) => f,
        Err(_) => return item,
    };

    let fn_name = &parsed_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    let mut param_metas = Vec::new();

    for (i, input) in parsed_fn.sig.inputs.iter().enumerate() {
        if let FnArg::Typed(pat_type) = input {
            let param_name = match &*pat_type.pat {
                Pat::Ident(pi) => pi.ident.to_string(),
                _ => format!("arg_{i}"),
            };

            if param_name == "space" || i == 0 {
                continue;
            }

            let type_str = quote!(#pat_type.ty).to_string();

            let is_mutable = match &*pat_type.ty {
                Type::Reference(tr) => tr.mutability.is_some(),
                _ => false,
            };

            let is_slice = type_str.contains('[') || type_str.contains("Slice");
            let is_atomic = type_str.contains("Atomic");
            let is_tile_mem = type_str.contains("TileMem");
            let is_ref = matches!(&*pat_type.ty, Type::Reference(_));

            let kind_token = if is_atomic {
                quote!(::enki::ExpectedParamKind::Atomic)
            } else if is_tile_mem {
                quote!(::enki::ExpectedParamKind::TileScratchpad)
            } else if is_slice {
                quote!(::enki::ExpectedParamKind::Slice)
            } else if !is_ref {
                quote!(::enki::ExpectedParamKind::ByValue)
            } else {
                quote!(::enki::ExpectedParamKind::PerCell)
            };

            param_metas.push(quote! {
                ::enki::NamParamMeta {
                    name: #param_name,
                    type_str: #type_str,
                    line: line!(),
                    column: column!(),
                    is_mutable: #is_mutable,
                    expected_kind: #kind_token,
                }
            });
        }
    }

    let contract_ident = quote::format_ident!("__ENKI_CONTRACT_{}", fn_name);

    let expanded = quote! {
        #[unsafe(no_mangle)]
        #[allow(non_snake_case)]
        #parsed_fn

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        #[unsafe(no_mangle)]
        pub static #contract_ident: ::enki::NamSignatureContract = ::enki::NamSignatureContract {
            nam_name: #fn_name_str,
            file_path: file!(),
            line: line!(),
            params: &[
                #(#param_metas),*
            ],
        };
    };

    TokenStream::from(expanded)
}
