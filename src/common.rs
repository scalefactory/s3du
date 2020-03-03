// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use std::str::FromStr;

// These are used by the CloudWatch and S3 modes.
pub type BucketNames = Vec<String>;

// BucketSizer trait implemented by cloudwatch and s3 mods
pub trait BucketSizer {
    fn bucket_size(&self, bucket: &str) -> Result<usize>;
    fn list_buckets(&mut self) -> Result<BucketNames>;
}

// Valid modes that s3du can operate in.
#[derive(Debug)]
pub enum ClientMode {
    CloudWatch,
    S3,
}

// This is used to work out which mode we're in after parsing the CLI.
// We shouldn't ever hit the error condition here.
impl FromStr for ClientMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cloudwatch" => Ok(Self::CloudWatch),
            "s3"         => Ok(Self::S3),
            _            => Err("no match"),
        }
    }
}
