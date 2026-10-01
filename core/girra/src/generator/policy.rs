#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzPolicy {
    pub name: String,
    pub allow_heap: bool,
    pub allow_recursion: bool,
    pub allow_panics: bool,
    pub allow_loops: bool,
    pub max_loop_iterations: usize,
    pub max_depth: usize,
    pub max_statements_per_block: usize,
    pub allow_structs: bool,
    pub allow_tuples: bool,
    pub allow_matches: bool,
    pub spmd_mode: bool,
}

impl FuzzPolicy {
    pub fn unconstrained() -> Self {
        Self {
            name: "unconstrained".into(),
            allow_heap: true,
            allow_recursion: true,
            allow_panics: true,
            allow_loops: true,
            max_loop_iterations: 32,
            max_depth: 8,
            max_statements_per_block: 6,
            allow_structs: true,
            allow_tuples: true,
            allow_matches: true,
            spmd_mode: false,
        }
    }

    pub fn gpu_legal() -> Self {
        Self {
            name: "gpu-legal".into(),
            allow_heap: false,
            allow_recursion: false,
            allow_panics: false,
            allow_loops: true,
            max_loop_iterations: 64,
            max_depth: 10,
            max_statements_per_block: 8,
            allow_structs: true,
            allow_tuples: true,
            allow_matches: true,
            spmd_mode: false,
        }
    }

    pub fn spmd_spatial() -> Self {
        Self {
            name: "spmd-spatial".into(),
            allow_heap: false,
            allow_recursion: false,
            allow_panics: false,
            allow_loops: true,
            max_loop_iterations: 64,
            max_depth: 12,
            max_statements_per_block: 10,
            allow_structs: true,
            allow_tuples: true,
            allow_matches: true,
            spmd_mode: true,
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "gpu-legal" | "legal" => Self::gpu_legal(),
            "spmd" | "spatial" => Self::spmd_spatial(),
            _ => Self::unconstrained(),
        }
    }
}
