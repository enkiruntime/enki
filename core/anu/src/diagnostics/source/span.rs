use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Role {
    #[default]
    Primary,
    Secondary,
}

impl From<u32> for Role {
    #[inline]
    fn from(val: u32) -> Self {
        match val {
            0 => Self::Primary,
            _ => Self::Secondary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub length: usize,
    pub role: Role,
}

impl Span {
    pub fn new<P: Into<PathBuf>>(
        file_path: P,
        line: usize,
        column: usize,
        length: usize,
        role: Role,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            line,
            column,
            length: length.max(1),
            role,
        }
    }

    #[inline]
    pub fn primary<P: Into<PathBuf>>(
        file_path: P,
        line: usize,
        column: usize,
        length: usize,
    ) -> Self {
        Self::new(file_path, line, column, length, Role::Primary)
    }

    #[inline]
    pub fn secondary<P: Into<PathBuf>>(
        file_path: P,
        line: usize,
        column: usize,
        length: usize,
    ) -> Self {
        Self::new(file_path, line, column, length, Role::Secondary)
    }

    pub fn from_coords<P: Into<PathBuf>>(
        file_path: P,
        line: u32,
        column: u32,
        length: u32,
        role: u32,
    ) -> Self {
        Self::new(
            file_path,
            line as usize,
            column as usize,
            length as usize,
            Role::from(role),
        )
    }

    #[inline(always)]
    pub fn is_primary(&self) -> bool {
        matches!(self.role, Role::Primary)
    }

    #[inline(always)]
    pub fn is_secondary(&self) -> bool {
        matches!(self.role, Role::Secondary)
    }
}
