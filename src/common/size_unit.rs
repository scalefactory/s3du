// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use std::str::FromStr;

#[derive(Debug)]
pub enum SizeUnit {
    Binary,
    Bytes,
    Decimal,
}

impl FromStr for SizeUnit {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "binary"  => Ok(Self::Binary),
            "bytes"   => Ok(Self::Bytes),
            "decimal" => Ok(Self::Decimal),
            _         => Err("no match"),
        }
    }
}
