// Common traits and types
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use rusoto_core::Region;
use super::ClientMode;

#[cfg(feature = "s3")]
use super::S3ObjectVersions;

// Client configuration
#[derive(Debug)]
pub struct ClientConfig {
    pub mode:   ClientMode,
    pub region: Region,
    #[cfg(feature = "s3")]
    pub s3_object_versions: S3ObjectVersions,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            mode:   ClientMode::CloudWatch,
            region: Region::UsEast1,
            #[cfg(feature = "s3")]
            s3_object_versions: S3ObjectVersions::Current,
        }
    }
}
