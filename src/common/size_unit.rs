// SizeUnit
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use humansize::file_size_opts::{
    self,
    FileSizeOpts,
};
use std::str::FromStr;

// We remove the space from the humansize output so that our own output is
// sortable by `sort -h`.
/// The same as `humansize::file_size_opts::BINARY` with `space` set to
/// `false`.
const SIZE_UNIT_BINARY: FileSizeOpts = FileSizeOpts {
    space: false,
    ..file_size_opts::BINARY
};

/// The same as `humansize::file_size_opts::DECIMAL` with `space` set to
/// `false`.
const SIZE_UNIT_DECIMAL: FileSizeOpts = FileSizeOpts {
    space: false,
    ..file_size_opts::DECIMAL
};

/// `SizeUnit` represents how we want the bucket sizes to be displayed.
#[derive(Debug)]
pub enum SizeUnit {
    /// Represent bucket sizes as human readable using SI units (multiples of
    /// 1024).
    Binary(FileSizeOpts),

    /// Represent bucket sizes as the number of bytes.
    Bytes,

    /// Represent bucket sizes as human readable using non-SI units (multiples
    /// of 1000).
    Decimal(FileSizeOpts),
}

/// This converts from the string arguments we receive on the command line to
/// our enum type.
impl FromStr for SizeUnit {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "binary"  => Ok(Self::Binary(SIZE_UNIT_BINARY)),
            "bytes"   => Ok(Self::Bytes),
            "decimal" => Ok(Self::Decimal(SIZE_UNIT_DECIMAL)),
            _         => Err("no match"),
        }
    }
}
