//! Memory-mapped account storage for high-performance access to validator state
//!
//! This module provides direct memory mapping of account data for ultra-low latency
//! access to validator state without going through the Geyser plugin interface.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use memmap2::{MmapMut, MmapOptions};
use agave_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPluginError, ReplicaAccountInfoVersions, Result as PluginResult,
};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug)]
pub struct MemoryMappedAccounts {
    mmap: Arc<RwLock<MmapMut>>,
    index: Arc<RwLock<HashMap<Pubkey, usize>>>,
    metadata: Arc<Mutex<MemoryMapMetadata>>,
    file_path: String,
}

#[derive(Debug)]
struct MemoryMapMetadata {
    current_position: usize,
    capacity: usize,
    account_count: usize,
    bytes_stored: usize,
}

impl MemoryMappedAccounts {
    pub fn new(file_path: &str, capacity: usize) -> PluginResult<Self> {
        let path = Path::new(file_path);
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;
        }
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;

        file.set_len(capacity as u64)
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?;

        let mmap = unsafe {
            MmapOptions::new()
                .len(capacity)
                .map_mut(&file)
                .map_err(|e| GeyserPluginError::Custom(Box::new(e)))?
        };

        tracing::info!(
            "Memory-mapped file created at {}, size: {} bytes",
            file_path, 
            capacity
        );

        let metadata = MemoryMapMetadata {
            current_position: 0,
            capacity,
            account_count: 0,
            bytes_stored: 0,
        };

        Ok(Self {
            mmap: Arc::new(RwLock::new(mmap)),
            index: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(Mutex::new(metadata)),
            file_path: file_path.to_owned(),
        })
    }

    pub fn store_account(
        &self,
        account: &ReplicaAccountInfoVersions,
        slot: u64,
    ) -> PluginResult<()> {
        let pubkey = match account {
            ReplicaAccountInfoVersions::V0_0_1(account_info) => {
                Pubkey::new_from_array(account_info.pubkey.try_into().map_err(|_| {
                    GeyserPluginError::AccountsUpdateError {
                        msg: "Invalid pubkey length".to_string(),
                    }
                })?)
            }
            ReplicaAccountInfoVersions::V0_0_2(account_info) => {
                Pubkey::new_from_array(account_info.pubkey.try_into().map_err(|_| {
                    GeyserPluginError::AccountsUpdateError {
                        msg: "Invalid pubkey length".to_string(),
                    }
                })?)
            }
            ReplicaAccountInfoVersions::V0_0_3(account_info) => {
                Pubkey::new_from_array(account_info.pubkey.try_into().map_err(|_| {
                    GeyserPluginError::AccountsUpdateError {
                        msg: "Invalid pubkey length".to_string(),
                    }
                })?)
            }
        };

        let (data_size, lamports, owner, executable) = match account {
            ReplicaAccountInfoVersions::V0_0_1(account_info) => (
                account_info.data.len(),
                account_info.lamports,
                Pubkey::new_from_array(account_info.owner.try_into().map_err(|_| {
                    GeyserPluginError::AccountsUpdateError {
                        msg: "Invalid owner length".to_string(),
                    }
                })?),
                account_info.executable,
            ),
            ReplicaAccountInfoVersions::V0_0_2(account_info) => (
                account_info.data.len(),
                account_info.lamports,
                Pubkey::new_from_array(account_info.owner.try_into().map_err(|_| {
                    GeyserPluginError::AccountsUpdateError {
                        msg: "Invalid owner length".to_string(),
                    }
                })?),
                account_info.executable,
            ),
            ReplicaAccountInfoVersions::V0_0_3(account_info) => (
                account_info.data.len(),
                account_info.lamports,
                Pubkey::new_from_array(account_info.owner.try_into().map_err(|_| {
                    GeyserPluginError::AccountsUpdateError {
                        msg: "Invalid owner length".to_string(),
                    }
                })?),
                account_info.executable,
            ),
        };

        let record_size = 8 + 32 + 32 + 8 + 1 + 4 + data_size;

        let offset = {
        let offset = {
            let mut metadata = self.metadata.lock().unwrap();
            let offset = metadata.current_position;
            
            if offset + record_size > metadata.capacity {
                return Err(GeyserPluginError::AccountsUpdateError {
                    msg: "Not enough space in memory-mapped file".to_string(),
                });
            }
            
            metadata.current_position += record_size;
            metadata.account_count += 1;
            metadata.bytes_stored += record_size;
            offset
        };

        {
            let mut mmap = self.mmap.write().unwrap();
            let mut position = offset;

            mmap[position..position + 8].copy_from_slice(&slot.to_le_bytes());
            position += 8;

            mmap[position..position + 32].copy_from_slice(pubkey.as_ref());
            position += 32;

            mmap[position..position + 32].copy_from_slice(owner.as_ref());
            position += 32;

            mmap[position..position + 8].copy_from_slice(&lamports.to_le_bytes());
            position += 8;

            mmap[position] = if executable { 1 } else { 0 };
            position += 1;

            let data_len = data_size as u32;
            mmap[position..position + 4].copy_from_slice(&data_len.to_le_bytes());
            position += 4;

            match account {
                ReplicaAccountInfoVersions::V0_0_1(account_info) => {
                    mmap[position..position + data_size].copy_from_slice(account_info.data);
                }
                ReplicaAccountInfoVersions::V0_0_2(account_info) => {
                    mmap[position..position + data_size].copy_from_slice(account_info.data);
                }
                ReplicaAccountInfoVersions::V0_0_3(account_info) => {
                    mmap[position..position + data_size].copy_from_slice(account_info.data);
                }
            }
        }

        {
            let mut index = self.index.write().unwrap();
            index.insert(pubkey, offset);
        }

        Ok(())
    }

    pub fn read_account(&self, pubkey: &Pubkey) -> PluginResult<Option<(u64, Vec<u8>)>> {
        let offset = {
            let index = self.index.read().unwrap();
            match index.get(pubkey) {
                Some(&offset) => offset,
                None => return Ok(None),
            }
        };

        let mmap = self.mmap.read().unwrap();
        
        let mut slot_bytes = [0u8; 8];
        slot_bytes.copy_from_slice(&mmap[offset..offset + 8]);
        let slot = u64::from_le_bytes(slot_bytes);
        
        let mut position = offset + 8 + 32 + 32;
        
        position += 8;
        
        position += 1;
        
        let mut data_len_bytes = [0u8; 4];
        data_len_bytes.copy_from_slice(&mmap[position..position + 4]);
        let data_len = u32::from_le_bytes(data_len_bytes) as usize;
        position += 4;
        
        let data = mmap[position..position + data_len].to_vec();
        
        Ok(Some((slot, data)))
    }

    pub fn flush(&self) -> PluginResult<()> {
        self.mmap.write().unwrap().flush()
            .map_err(|e| GeyserPluginError::Custom(Box::new(e)))
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let metadata = self.metadata.lock().unwrap();
        (metadata.account_count, metadata.bytes_stored, metadata.capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo;
    use tempfile::tempdir;

    #[test]
    fn test_memory_mapped_accounts() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_mmap.bin").to_str().unwrap().to_owned();
        
        let mmap_capacity = 1024 * 1024;
        let mmap = MemoryMappedAccounts::new(&file_path, mmap_capacity).unwrap();
        
        let pubkey = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let lamports = 12345;
        let data = vec![1, 2, 3, 4, 5];
        let executable = false;
        let rent_epoch = 0;
        
        let account_info = ReplicaAccountInfo {
            pubkey: pubkey.as_ref(),
            owner: owner.as_ref(),
            lamports,
            data: &data,
            executable,
            rent_epoch,
            write_version: 1,
        };
        
        let account = ReplicaAccountInfoVersions::V0_0_1(&account_info);
        let slot = 42;
        
        mmap.store_account(&account, slot).unwrap();
        
        let result = mmap.read_account(&pubkey).unwrap();
        assert!(result.is_some());
        
        let (read_slot, read_data) = result.unwrap();
        assert_eq!(read_slot, slot);
        assert_eq!(read_data, data);
        
        let (account_count, bytes_stored, capacity) = mmap.stats();
        assert_eq!(account_count, 1);
        assert!(bytes_stored > 0);
        assert_eq!(capacity, mmap_capacity);
    }
}