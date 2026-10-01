//! Module: girra/src/types/mod.rs
//!
//! Unified Type System representation for Girra AST Synthesis.

pub mod composite;
pub mod lifetime;
pub mod primitive;

pub use composite::{ArrayType, HeapType, ReferenceType, StructField, StructType, TupleType};
pub use lifetime::{LifetimeId, format_generics};
pub use primitive::{FloatType, IntType, PrimitiveType, UIntType};

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnershipModel {
    Copy,
    Move,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Struct(StructType),
    Tuple(TupleType),
    Array(ArrayType),
    Reference(ReferenceType),
    Option(Box<Type>),
    Heap(HeapType),
    String,
}

impl Type {
    pub fn i8() -> Self {
        Self::Primitive(PrimitiveType::Int(IntType::I8))
    }
    pub fn i16() -> Self {
        Self::Primitive(PrimitiveType::Int(IntType::I16))
    }
    pub fn i32() -> Self {
        Self::Primitive(PrimitiveType::Int(IntType::I32))
    }
    pub fn i64() -> Self {
        Self::Primitive(PrimitiveType::Int(IntType::I64))
    }
    pub fn u8() -> Self {
        Self::Primitive(PrimitiveType::UInt(UIntType::U8))
    }
    pub fn u16() -> Self {
        Self::Primitive(PrimitiveType::UInt(UIntType::U16))
    }
    pub fn u32() -> Self {
        Self::Primitive(PrimitiveType::UInt(UIntType::U32))
    }
    pub fn u64() -> Self {
        Self::Primitive(PrimitiveType::UInt(UIntType::U64))
    }
    pub fn usize_t() -> Self {
        Self::Primitive(PrimitiveType::UInt(UIntType::USize))
    }
    pub fn f32() -> Self {
        Self::Primitive(PrimitiveType::Float(FloatType::F32))
    }
    pub fn f64() -> Self {
        Self::Primitive(PrimitiveType::Float(FloatType::F64))
    }
    pub fn bool() -> Self {
        Self::Primitive(PrimitiveType::Bool)
    }
    pub fn unit() -> Self {
        Self::Primitive(PrimitiveType::Unit)
    }

    pub fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::Primitive(p) => p.to_token_stream(),
            Self::Struct(s) => s.to_token_stream(),
            Self::Tuple(t) => t.to_token_stream(),
            Self::Array(a) => a.to_token_stream(),
            Self::Reference(r) => r.to_token_stream(),
            Self::Option(inner) => {
                let elem = inner.to_token_stream();
                quote! { Option<#elem> }
            }
            Self::Heap(h) => h.to_token_stream(),
            Self::String => quote! { String },
        }
    }

    pub fn ownership_model(&self) -> OwnershipModel {
        match self {
            Self::Primitive(_) => OwnershipModel::Copy,
            Self::Reference(r) => {
                if r.mutable {
                    OwnershipModel::Move
                } else {
                    OwnershipModel::Copy
                }
            }
            Self::Tuple(t) => {
                if t.elements
                    .iter()
                    .all(|e| e.ownership_model() == OwnershipModel::Copy)
                {
                    OwnershipModel::Copy
                } else {
                    OwnershipModel::Move
                }
            }
            Self::Array(a) => a.element_type.ownership_model(),
            Self::Struct(_) | Self::Option(_) | Self::Heap(_) | Self::String => {
                OwnershipModel::Move
            }
        }
    }

    pub fn is_copy(&self) -> bool {
        self.ownership_model() == OwnershipModel::Copy
    }

    pub fn member_types(&self) -> Vec<Type> {
        let mut members = vec![self.clone()];
        match self {
            Self::Struct(s) => {
                for f in &s.fields {
                    members.extend(f.ty.member_types());
                }
            }
            Self::Tuple(t) => {
                for elem in &t.elements {
                    members.extend(elem.member_types());
                }
            }
            Self::Array(a) => members.extend(a.element_type.member_types()),
            Self::Reference(r) => members.extend(r.element_type.member_types()),
            Self::Option(inner) => members.extend(inner.member_types()),
            Self::Heap(HeapType::Vec(inner) | HeapType::Box(inner)) => {
                members.extend(inner.member_types())
            }
            Self::Primitive(_) | Self::String => {}
        }
        members
    }

    pub fn lifetime_parameters(&self) -> BTreeSet<LifetimeId> {
        let mut set = BTreeSet::new();
        match self {
            Self::Reference(r) => {
                if let Some(id) = r.lifetime_id {
                    set.insert(LifetimeId(id));
                }
                set.extend(r.element_type.lifetime_parameters());
            }
            Self::Struct(s) => {
                for f in &s.fields {
                    set.extend(f.ty.lifetime_parameters());
                }
            }
            Self::Tuple(t) => {
                for elem in &t.elements {
                    set.extend(elem.lifetime_parameters());
                }
            }
            Self::Array(a) => set.extend(a.element_type.lifetime_parameters()),
            Self::Option(inner) => set.extend(inner.lifetime_parameters()),
            Self::Heap(HeapType::Vec(inner) | HeapType::Box(inner)) => {
                set.extend(inner.lifetime_parameters())
            }
            Self::Primitive(_) | Self::String => {}
        }
        set
    }

    pub fn is_integer(&self) -> bool {
        match self {
            Self::Primitive(p) => p.is_integer(),
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            Self::Primitive(p) => p.is_float(),
            _ => false,
        }
    }

    pub fn is_numeric(&self) -> bool {
        match self {
            Self::Primitive(p) => p.is_numeric(),
            _ => false,
        }
    }

    pub fn is_bitwise_compatible(&self) -> bool {
        match self {
            Self::Primitive(p) => p.is_bitwise_compatible(),
            _ => false,
        }
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, Self::Reference(_))
    }

    pub fn is_heap(&self) -> bool {
        matches!(self, Self::Heap(_) | Self::String)
    }
    pub fn zero_expr(&self) -> crate::ast::expr::Expr {
        use crate::ast::expr::Expr;
        match self {
            Self::Primitive(PrimitiveType::Int(IntType::I8)) => Expr::Int(0, "i8"),
            Self::Primitive(PrimitiveType::Int(IntType::I16)) => Expr::Int(0, "i16"),
            Self::Primitive(PrimitiveType::Int(IntType::I32)) => Expr::Int(0, "i32"),
            Self::Primitive(PrimitiveType::Int(IntType::I64)) => Expr::Int(0, "i64"),
            Self::Primitive(PrimitiveType::UInt(UIntType::U8)) => Expr::UInt(0, "u8"),
            Self::Primitive(PrimitiveType::UInt(UIntType::U16)) => Expr::UInt(0, "u16"),
            Self::Primitive(PrimitiveType::UInt(UIntType::U32)) => Expr::UInt(0, "u32"),
            Self::Primitive(PrimitiveType::UInt(UIntType::U64)) => Expr::UInt(0, "u64"),
            Self::Primitive(PrimitiveType::UInt(UIntType::USize)) => Expr::UInt(0, "usize"),
            Self::Primitive(PrimitiveType::Float(FloatType::F32)) => {
                Expr::Float("0.0".into(), "f32")
            }
            Self::Primitive(PrimitiveType::Float(FloatType::F64)) => {
                Expr::Float("0.0".into(), "f64")
            }
            Self::Primitive(PrimitiveType::Bool) => Expr::Bool(false),
            _ => Expr::Int(0, "i32"),
        }
    }

    pub fn zero_literal_tokens(&self) -> TokenStream {
        match self {
            Self::Primitive(PrimitiveType::Int(IntType::I8)) => quote! { 0i8 },
            Self::Primitive(PrimitiveType::Int(IntType::I16)) => quote! { 0i16 },
            Self::Primitive(PrimitiveType::Int(IntType::I32)) => quote! { 0i32 },
            Self::Primitive(PrimitiveType::Int(IntType::I64)) => quote! { 0i64 },
            Self::Primitive(PrimitiveType::Int(IntType::I128)) => quote! { 0i128 },
            Self::Primitive(PrimitiveType::Int(IntType::ISize)) => quote! { 0isize },
            Self::Primitive(PrimitiveType::UInt(UIntType::U8)) => quote! { 0u8 },
            Self::Primitive(PrimitiveType::UInt(UIntType::U16)) => quote! { 0u16 },
            Self::Primitive(PrimitiveType::UInt(UIntType::U32)) => quote! { 0u32 },
            Self::Primitive(PrimitiveType::UInt(UIntType::U64)) => quote! { 0u64 },
            Self::Primitive(PrimitiveType::UInt(UIntType::U128)) => quote! { 0u128 },
            Self::Primitive(PrimitiveType::UInt(UIntType::USize)) => quote! { 0usize },
            Self::Primitive(PrimitiveType::Float(FloatType::F32)) => quote! { 0.0f32 },
            Self::Primitive(PrimitiveType::Float(FloatType::F64)) => quote! { 0.0f64 },
            Self::Primitive(PrimitiveType::Bool) => quote! { false },
            Self::Primitive(PrimitiveType::Unit) => quote! { () },
            _ => quote! { Default::default() },
        }
    }
}
