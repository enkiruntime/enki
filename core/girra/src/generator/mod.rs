pub mod ast_gen;
pub mod ident;
pub mod policy;
pub mod weights;

pub use ast_gen::ASTGenerator;
pub use ident::IdentGenerator;
pub use policy::FuzzPolicy;
pub use weights::SelectionWeights;

use crate::ast::Program;

pub fn generate_program(seed: u64, policy: FuzzPolicy) -> Program {
    let mut generator = ASTGenerator::new(seed, policy);
    generator.generate_program(seed)
}
