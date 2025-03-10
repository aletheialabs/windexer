//! Tests for the wIndexer Geyser plugin
//!
//! This module contains tests for the plugin implementation.

#[cfg(test)]
mod tests {
    use crate::{ShutdownFlag, PluginVersion};

    #[test]
    fn test_plugin_version() {
        let version = PluginVersion::new();
        assert!(!version.version.is_empty());
        assert!(version.build_timestamp > 0);
        assert!(!version.rust_version.is_empty());
    }
    
    #[test]
    fn test_shutdown_flag() {
        let flag = ShutdownFlag::new();
        assert!(!flag.is_shutdown());
        flag.shutdown();
        assert!(flag.is_shutdown());
    }
} 