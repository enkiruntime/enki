pub mod ownership;
pub mod registry;
pub mod table;

pub use ownership::{IdentifierData, OwnershipState};
pub use registry::{FunctionSignature, GlobalRegistry};
pub use table::SymbolTable;
