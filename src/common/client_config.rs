// ClientConfig
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use super::{
    ClientMode,
    Region,
};

#[cfg(feature = "s3")]
use super::ObjectVersions;

/// Client configuration.
#[derive(Debug)]
pub struct ClientConfig {
    /// The bucket name that the client should report the size of.
    ///
    /// If this isn't given, all discovered S3 buckets will have their sizes
    /// reported.
    pub bucket_name: Option<String>,

    /// The mode that `s3du` will run in.
    ///
    /// This selects which AWS client will be used.
    pub mode: ClientMode,

    /// The region that our AWS client should be created in.
    ///
    /// This will affect bucket discovery.
    pub region: Region,

    /// The S3 object versions that should be used when calculating the bucket
    /// size.
    ///
    /// This only has an effect when running in S3 mode and the field will only
    /// be present when compiled with the `s3` feature.
    #[cfg(feature = "s3")]
    pub object_versions: ObjectVersions,
}

impl Default for ClientConfig {
    /// Returns a default `ClientConfig`.
    ///
    /// If compiled with the `cloudwatch` feature, `CloudWatch` will be the
    /// default `ClientMode`, otherwise `S3` will be the default.
    ///
    /// If compiled without the `s3` feature, the `object_versions` field
    /// will be absent.
    ///
    /// ```rust
    /// ClientConfig {
    ///     bucket_name:     None,
    ///     mode:            ClientMode::CloudWatch,
    ///     region:          Region::new(),
    ///     object_versions: ObjectVersions::Current,
    /// }
    /// ```
    fn default() -> Self {
        #[cfg(feature = "cloudwatch")]
        let mode = ClientMode::CloudWatch;

        #[cfg(all(feature = "s3", not(feature = "cloudwatch")))]
        let mode = ClientMode::S3;

        Self {
            bucket_name: None,
            mode:        mode,
            region:      Region::new(),
            #[cfg(feature = "s3")]
            object_versions: ObjectVersions::Current,
        }
    }
}
