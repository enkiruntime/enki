use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedParamKind {
    PerCell,
    Slice,
    Atomic,
    TileScratchpad,
    ByValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamParamMeta {
    pub name: &'static str,
    pub type_str: &'static str,
    pub line: u32,
    pub column: u32,
    pub is_mutable: bool,
    pub expected_kind: ExpectedParamKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamSignatureContract {
    pub nam_name: &'static str,
    pub file_path: &'static str,
    pub line: u32,
    pub params: &'static [NamParamMeta],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DispatchSafetyMode {
    #[default]
    Safe,
    Unchecked,
}

impl DispatchSafetyMode {
    #[inline(always)]
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Safe)
    }

    #[inline(always)]
    pub fn is_unchecked(&self) -> bool {
        matches!(self, Self::Unchecked)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceAccessKind {
    PerCell {
        element_count: usize,
    },
    Slice {
        element_range: Range<usize>,
        total_container_len: usize,
    },
    Atomic {
        element_count: usize,
    },
    TileScratchpad {
        element_count: usize,
    },
    ValueUniform {
        byte_size: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputResourceRecord {
    pub arg_index: usize,
    pub root_id: Option<usize>,
    pub access_kind: ResourceAccessKind,
    pub is_mutable: bool,
    pub element_type_name: &'static str,
}

impl InputResourceRecord {
    #[inline]
    pub fn as_slice_range(&self) -> Option<&Range<usize>> {
        match &self.access_kind {
            ResourceAccessKind::Slice { element_range, .. } => Some(element_range),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn is_write(&self) -> bool {
        self.is_mutable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceContract {
    pub dimensions: (usize, usize, usize),
    pub total_cells: usize,
}

impl SpaceContract {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        let x = x.max(1);
        let y = y.max(1);
        let z = z.max(1);
        let total_cells = x * y * z;

        Self {
            dimensions: (x, y, z),
            total_cells,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NamDispatchMap {
    pub nam_name: String,

    pub safety_mode: DispatchSafetyMode,

    pub space: SpaceContract,

    pub inputs: Vec<InputResourceRecord>,

    pub call_site: Option<(&'static str, u32, u32)>,

    pub expected_contract: Option<&'static NamSignatureContract>,
}

impl NamDispatchMap {
    pub fn new(
        nam_name: String,
        safety_mode: DispatchSafetyMode,
        space: SpaceContract,
        call_site: Option<(&'static str, u32, u32)>,
    ) -> Self {
        Self {
            nam_name,
            safety_mode,
            space,
            inputs: Vec::with_capacity(8),
            call_site,
            expected_contract: None,
        }
    }

    #[inline]
    pub fn with_contract(mut self, contract: &'static NamSignatureContract) -> Self {
        self.expected_contract = Some(contract);
        self
    }

    pub fn push_input(&mut self, input: InputResourceRecord) {
        self.inputs.push(input);
    }

    pub fn inputs_for_root(
        &self,
        target_root: usize,
    ) -> impl Iterator<Item = &InputResourceRecord> {
        self.inputs
            .iter()
            .filter(move |i| i.root_id == Some(target_root))
    }

    pub fn has_mutable_inputs(&self) -> bool {
        self.inputs.iter().any(|i| i.is_mutable)
    }
}
