// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bucket_sizer;
mod client_config;
mod client_mode;
mod s3_object_versions;
mod size_unit;

pub use bucket_sizer::*;
pub use client_config::*;
pub use client_mode::*;
pub use s3_object_versions::*;
pub use size_unit::*;

// These are used by the CloudWatch and S3 modes.
pub type BucketNames = Vec<String>;
