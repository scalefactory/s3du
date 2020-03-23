// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use anyhow::Result;
use async_trait::async_trait;

use super::BucketNames;

// BucketSizer trait implemented by cloudwatch and s3 mods
#[async_trait]
pub trait BucketSizer {
    // Takes a bucket name and returns the bucket size in bytes
    async fn bucket_size(&self, bucket: &str) -> Result<usize>;
    // Returns a list of bucket names
    async fn list_buckets(&mut self) -> Result<BucketNames>;
}
