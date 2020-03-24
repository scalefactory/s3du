// Common traits and types
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
const SIZE_UNIT_BINARY: FileSizeOpts = FileSizeOpts {
    space:      false,
    ..file_size_opts::BINARY
};

const SIZE_UNIT_DECIMAL: FileSizeOpts = FileSizeOpts {
    space:      false,
    ..file_size_opts::DECIMAL
};

#[derive(Debug)]
pub enum SizeUnit {
    Binary(FileSizeOpts),
    Bytes,
    Decimal(FileSizeOpts),
}

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
