//! Utility functions and helpers

mod crypto;
mod time;

pub use crypto::{hash_message, verify_signature};
pub use time::{current_timestamp, duration_since};

use solana_sdk::pubkey::Pubkey;

pub fn pubkey_to_string(pubkey: &Pubkey) -> String {
    pubkey.to_string()
}

pub fn string_to_pubkey(s: &str) -> crate::Result<Pubkey> {
    s.parse().map_err(|e| crate::Error::Other(e.to_string()))
}