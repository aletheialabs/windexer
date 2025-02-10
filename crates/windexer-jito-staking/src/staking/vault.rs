// crates/windexer-jito-staking/src/staking/vault.rs

use solana_sdk::pubkey::Pubkey;
use anyhow::Result;

pub struct VaultManager {
    vaults: Vec<Pubkey>,
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            vaults: Vec::new()
        }
    }

    pub async fn create_vault(
        &mut self,
        _admin: Pubkey,
        _mint: Pubkey,
        _ncn: Pubkey
    ) -> Result<Pubkey> {
        let vault = Pubkey::new_unique();
        self.vaults.push(vault);
        Ok(vault)
    }

    pub async fn add_delegation(
        &self,
        vault: Pubkey,
        _operator: Pubkey,
        _amount: u64
    ) -> Result<()> {
        if !self.vaults.contains(&vault) {
            return Err(anyhow::anyhow!("Invalid vault"));
        }
        Ok(())
    }
}