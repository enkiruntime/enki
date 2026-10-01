use proc_macro2::TokenStream;
use quote::quote;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntType {
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
}

impl IntType {
    pub fn to_token_stream(self) -> TokenStream {
        match self {
            Self::I8 => quote! { i8 },
            Self::I16 => quote! { i16 },
            Self::I32 => quote! { i32 },
            Self::I64 => quote! { i64 },
            Self::I128 => quote! { i128 },
            Self::ISize => quote! { isize },
        }
    }

    pub fn bit_width(self) -> usize {
        match self {
            Self::I8 => 8,
            Self::I16 => 16,
            Self::I32 => 32,
            Self::I64 => 64,
            Self::I128 => 128,
            Self::ISize => 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UIntType {
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
}

impl UIntType {
    pub fn to_token_stream(self) -> TokenStream {
        match self {
            Self::U8 => quote! { u8 },
            Self::U16 => quote! { u16 },
            Self::U32 => quote! { u32 },
            Self::U64 => quote! { u64 },
            Self::U128 => quote! { u128 },
            Self::USize => quote! { usize },
        }
    }

    pub fn bit_width(self) -> usize {
        match self {
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::U64 => 64,
            Self::U128 => 128,
            Self::USize => 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatType {
    F32,
    F64,
}

impl FloatType {
    pub fn to_token_stream(self) -> TokenStream {
        match self {
            Self::F32 => quote! { f32 },
            Self::F64 => quote! { f64 },
        }
    }

    pub fn bit_width(self) -> usize {
        match self {
            Self::F32 => 32,
            Self::F64 => 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Int(IntType),
    UInt(UIntType),
    Float(FloatType),
    Bool,
    Unit,
}

impl PrimitiveType {
    pub fn to_token_stream(self) -> TokenStream {
        match self {
            Self::Int(t) => t.to_token_stream(),
            Self::UInt(t) => t.to_token_stream(),
            Self::Float(t) => t.to_token_stream(),
            Self::Bool => quote! { bool },
            Self::Unit => quote! { () },
        }
    }

    pub fn is_integer(self) -> bool {
        matches!(self, Self::Int(_) | Self::UInt(_))
    }

    pub fn is_float(self) -> bool {
        matches!(self, Self::Float(_))
    }

    pub fn is_numeric(self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn is_bitwise_compatible(self) -> bool {
        self.is_integer() || matches!(self, Self::Bool)
    }

    pub fn byte_size(self) -> usize {
        match self {
            Self::Int(t) => t.bit_width() / 8,
            Self::UInt(t) => t.bit_width() / 8,
            Self::Float(t) => t.bit_width() / 8,
            Self::Bool => 1,
            Self::Unit => 0,
        }
    }
}
