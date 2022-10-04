// HumanSize trait and implementations
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use humansize::format_size;
use log::debug;
use super::SizeUnit;

/// `HumanSize` trait.
pub trait HumanSize {
    fn humansize(&self, unit: &SizeUnit) -> String;
}

/// `HumanSize` trait implementation for `usize`.
impl HumanSize for u64 {
    /// Return `self` as a human friendly size if requested by `unit`.
    fn humansize(&self, unit: &SizeUnit) -> String {
        debug!("humansize: size {}, unit {:?}", self, unit);

        // Unwrap should be fine here, usize cannot be negative, so file_size
        // shouldn't error.
        match unit {
            SizeUnit::Binary(unit)  => format_size(*self, unit),
            SizeUnit::Bytes         => self.to_string(),
            SizeUnit::Decimal(unit) => format_size(*self, unit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    #[test]
    fn test_humansize() {
        let tests = vec![
            (0,    "binary",  "0B"),
            (1024, "binary",  "1KiB"),
            (1,    "bytes",   "1"),
            (1024, "decimal", "1.02kB"),
        ];

        for test in tests {
            let size: u64 = test.0;
            let unit      = SizeUnit::from_str(test.1).unwrap();
            let expected  = test.2;

            let ret = size.humansize(&unit);

            assert_eq!(ret, expected);
        }
    }
}
