// Handles region things
use aws_config::meta::region::future;
use aws_config::meta::region::ProvideRegion;
use aws_types::region;
use std::env;
use tracing::debug;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Region {
    region: Option<region::Region>,
}

impl Region {
    pub fn new() -> Self {
        // By default, we try to get a region from the environment, this might
        // be overridden later depending on CLI options.
        let possibilities = [
            env::var("AWS_REGION"),
            env::var("AWS_DEFAULT_REGION"),
        ];

        let region = possibilities
            .iter()
            .find_map(|region| region.as_ref().ok())
            .map(|region| region::Region::new(region.clone()));

        debug!("AWS_REGION in environment is: {:?}", region);

        Self {
            region,
        }
    }

    // Returns the region name
    pub fn name(&self) -> &str {
        match &self.region {
            Some(region) => region.as_ref(),
            None         => "default",
        }
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
        future::ProvideRegion::ready(self.region.clone())
    }
}

impl ProvideRegion for &Region {
    // Takes our region string and returns a proper AWS Region, this should
    // allow us to pass our Region into AWS SDK functions expecting an AWS
    // Region.
    fn region(&self) -> future::ProvideRegion {
        future::ProvideRegion::ready(self.region.clone())
    }
}
