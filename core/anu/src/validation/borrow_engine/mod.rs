pub mod contract;
pub mod spatial;
pub mod temporal;
pub mod violation;

pub use contract::ContractMatcher;
pub use spatial::SpatialBorrowChecker;
pub use temporal::FrameBorrowLedger;
pub use violation::BorrowViolation;

use crate::nam_args_api::NamDispatchMap;

pub struct BorrowEngine;

impl BorrowEngine {
    pub fn validate_dispatch(
        map: &NamDispatchMap,
        ledger: &mut FrameBorrowLedger,
    ) -> Result<(), BorrowViolation> {
        ContractMatcher::verify(map)?;

        SpatialBorrowChecker::verify(map)?;

        ledger.record_dispatch(map)?;

        Ok(())
    }
}
