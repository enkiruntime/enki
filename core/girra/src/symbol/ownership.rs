use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnershipState {
    Valid,
    Borrowed,
    MutablyBorrowed,
    PartiallyValid,
    Invalid,
}

impl OwnershipState {
    #[inline]
    pub fn is_movable(self) -> bool {
        self == Self::Valid
    }

    #[inline]
    pub fn is_borrowable(self) -> bool {
        matches!(self, Self::Valid | Self::Borrowed)
    }

    #[inline]
    pub fn is_mutably_borrowable(self) -> bool {
        self == Self::Valid
    }

    #[inline]
    pub fn is_assignable(self) -> bool {
        matches!(self, Self::Valid | Self::PartiallyValid | Self::Invalid)
    }

    #[inline]
    pub fn is_overriding(self) -> bool {
        matches!(self, Self::Invalid | Self::PartiallyValid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierData {
    pub name: String,
    pub ty: Type,
    pub is_mutable: bool,
    pub validity: OwnershipState,
    pub depth: usize,
    pub is_constant: bool,
}

impl IdentifierData {
    pub fn new(name: impl Into<String>, ty: Type, is_mutable: bool, depth: usize) -> Self {
        Self {
            name: name.into(),
            ty,
            is_mutable,
            validity: OwnershipState::Valid,
            depth,
            is_constant: false,
        }
    }

    pub fn new_const(name: impl Into<String>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
            is_mutable: false,
            validity: OwnershipState::Valid,
            depth: 0,
            is_constant: true,
        }
    }
}
