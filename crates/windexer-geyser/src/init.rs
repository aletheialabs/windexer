//! Initialize the Geyser plugin.
use chrono::{Utc, Duration};

pub fn init() -> Result<(), std::io::Error> {
    log::info("Geyser plugin initialized");

    core::init();
    
    Ok(())
}