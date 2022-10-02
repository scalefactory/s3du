// Handles region things
use aws_config::meta::region::future;
use aws_config::meta::region::ProvideRegion;
use aws_types::region;
use log::debug;
use std::env;

#[cfg(feature = "s3")]
use aws_smithy_http::endpoint::Endpoint;

#[derive(Clone, Debug, Default)]
pub struct Region {
    region:   Option<region::Region>,

    #[cfg(feature = "s3")]
    endpoint: Option<Endpoint>,
}

// Endpoint doesn't impl PartialEq, do our own thing
impl PartialEq for Region {
    fn eq(&self, other: &Self) -> bool {
        let eq_region = self.region == other.region;
        let eq_endpoint = true;

        eq_region && eq_endpoint
    }
}

impl Region {
    pub fn new() -> Self {
        // By default, we try to get a region from the environment, this might
        // be overridden later depending on CLI options.
        let possibilities = vec![
            env::var("AWS_REGION"),
            env::var("AWS_DEFAULT_REGION"),
        ];

        let region = possibilities
            .iter()
            .find_map(|region| region.as_ref().ok())
            .map(|region| region::Region::new(region.to_owned()));

        debug!("AWS_REGION in environment is: {:?}", region);

        Self {
            region: region,
            ..Default::default()
        }
    }

    // Returns the region name
    pub fn name(&self) -> &str {
        match &self.region {
            Some(region) => region.as_ref(),
            None         => "default",
        }
    }

    #[cfg(feature = "s3")]
    pub fn set_endpoint(mut self, endpoint: Endpoint) -> Self {
        debug!("Region endpoint set to: {:?}", endpoint);

        self.endpoint = Some(endpoint);
        self
    }

    pub fn set_region(mut self, region: &str) -> Self {
        debug!("Region set to: {:?}", region);

        let region = region::Region::new(region.to_string());
        self.region = Some(region);
        self
    }
}

impl ProvideRegion for Region {
    // Takes our region string and returns a proper AWS Region, this should
    // allow us to pass our Region into AWS SDK functions expecting an AWS
    // Region.
    fn region(&self) -> future::ProvideRegion {
        future::ProvideRegion::ready(self.region.to_owned())
    }
}

impl ProvideRegion for &Region {
    // Takes our region string and returns a proper AWS Region, this should
    // allow us to pass our Region into AWS SDK functions expecting an AWS
    // Region.
    fn region(&self) -> future::ProvideRegion {
        future::ProvideRegion::ready(self.region.to_owned())
    }
}
