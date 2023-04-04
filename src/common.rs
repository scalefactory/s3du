// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The `Bucket` struct
mod bucket;

/// The `BucketSizer` trait.
mod bucket_sizer;

/// `ClientConfig` holds configuration for the `CloudWatch` and S3 `Client`s.
mod client_config;

/// `ClientMode` enum is used to select which `Client` will be used.
mod client_mode;

/// `HumanSize` trait for `usize` used to output friendly bucket sizes.
mod human_size;

/// `Region` struct wraps a basic string and allows us to return appropriate
/// AWS types when needed.
mod region;

/// `SizeUnit` enum is used to select how the bucket sizes will be output.
mod size_unit;

/// `ObjectVersions` selects which S3 objects will be used when summing the
/// size of the buckets.
#[cfg(feature = "s3")]
mod object_versions;

pub use bucket::*;
pub use bucket_sizer::*;
pub use client_config::*;
pub use client_mode::*;
pub use human_size::*;
pub use region::*;
pub use size_unit::*;

#[cfg(feature = "s3")]
pub use object_versions::*;

/// `BucketNames` is a convenience type used by both the `CloudWatch` and S3
/// clients.
pub type BucketNames = Vec<String>;
