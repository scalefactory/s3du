// Definition of a bucket
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use rusoto_core::Region;

/// Represents an S3 bucket.
///
/// This will always have a `name` and optionally a `Region`.
#[derive(Debug)]
pub struct Bucket {
    pub name:          String,
    pub region:        Option<Region>,
    pub storage_types: Option<Vec<String>>,
}

/// Convenience type for a list of `Bucket`.
pub type Buckets = Vec<Bucket>;
