// Handles region things
use aws_types::region::{
    self,
    EnvironmentProvider,
    ProvideRegion,
};
use log::debug;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Region {
    endpoint: Option<String>,
    region:   Option<region::Region>,
}

impl Region {
    pub fn new() -> Self {
        // By default, we try to get a region from the environment, this might
        // be overridden later depending on CLI options.
        let env_region = EnvironmentProvider::new().region();

        debug!("AWS_REGION in environment is: {:?}", env_region);

        Self {
            region: env_region,
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

    pub fn set_endpoint(mut self, endpoint: &str) -> Self {
        debug!("Region endpoint set to: {:?}", endpoint);

        self.endpoint = Some(endpoint.to_string());
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
    fn region(&self) -> Option<region::Region> {
        //self.region.map(|r| region::Region::new(r))
        self.region.to_owned()
    }
}

impl ProvideRegion for &Region {
    // Takes our region string and returns a proper AWS Region, this should
    // allow us to pass our Region into AWS SDK functions expecting an AWS
    // Region.
    fn region(&self) -> Option<region::Region> {
        //self.region.map(|r| region::Region::new(r))
        self.region.to_owned()
    }
}
