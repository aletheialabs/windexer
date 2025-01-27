//! Configuration for the Geyser plugin. 

use serde::{Deserialize, Serialize}; 

#[derive(Debug, Deserialize)]
pub struct Config {
    /// API endpoint URL for the Geyser service.
    geyser_url: String,
    /// Jito staking configuration parameters.
    jito_staking: Option[JitoStakingConfig],
} 

impl Config {
    pub fn default() -> Self {
        Config {
            geyser_url: "http://localhost:80 ",
            jito_staking: None,
        }
    }
} 

/// Loads the configuration from external sources (e.g., environment variables).
pub fn load_from_env() -> Result<Self, std::io::Error> {
    // Configuration loading logic here
    Ok(Config::default())
} 

/// Serialize and deserialize the configuration.
implSerialize()
implDeserialize()