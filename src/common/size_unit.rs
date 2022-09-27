// SizeUnit
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use humansize::{
    BINARY,
    DECIMAL,
    FormatSizeOptions,
};
use std::str::FromStr;

/// `SizeUnit` represents how we want the bucket sizes to be displayed.
#[derive(Debug)]
pub enum SizeUnit {
    /// Represent bucket sizes as human readable using SI units (multiples of
    /// 1024).
    Binary(FormatSizeOptions),

    /// Represent bucket sizes as the number of bytes.
    Bytes,

    /// Represent bucket sizes as human readable using non-SI units (multiples
    /// of 1000).
    Decimal(FormatSizeOptions),
}

/// This converts from the string arguments we receive on the command line to
/// our enum type.
/// We remove the space from the humansize output so that our own output is
/// sortable by `sort -h`.
impl FromStr for SizeUnit {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "binary"  => Ok(Self::Binary(BINARY.space_after_value(false))),
            "bytes"   => Ok(Self::Bytes),
            "decimal" => Ok(Self::Decimal(DECIMAL.space_after_value(false))),
            _         => Err("no match"),
        }
    }
}
