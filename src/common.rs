// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use async_trait::async_trait;

mod client_config;
pub use client_config::*;
mod client_mode;
pub use client_mode::*;
mod s3_object_versions;
pub use s3_object_versions::*;
mod size_unit;
pub use size_unit::*;

// These are used by the CloudWatch and S3 modes.
pub type BucketNames = Vec<String>;

// BucketSizer trait implemented by cloudwatch and s3 mods
#[async_trait]
pub trait BucketSizer {
    // Takes a bucket name and returns the bucket size in bytes
    async fn bucket_size(&self, bucket: &str) -> Result<usize>;
    // Returns a list of bucket names
    async fn list_buckets(&mut self) -> Result<BucketNames>;
}
