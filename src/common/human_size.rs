// HumanSize trait and implementations
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use crate::common::SizeUnit;
use humansize::FileSize;
use log::debug;

/// `HumanSize` trait.
pub trait HumanSize {
    fn humansize(&self, unit: &SizeUnit) -> String;
}

/// `HumanSize` trait implementation for `usize`.
impl HumanSize for usize {
    /// Return `self` as a human friendly size if requested by `unit`.
    fn humansize(&self, unit: &SizeUnit) -> String {
        debug!("humansize: size {}, unit {:?}", self, unit);

        match unit {
            SizeUnit::Binary(unit)  => self.file_size(unit).unwrap(),
            SizeUnit::Bytes         => self.to_string(),
            SizeUnit::Decimal(unit) => self.file_size(unit).unwrap(),
        }
    }
}
