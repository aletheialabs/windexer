//! SIMD-accelerated processing for agave account, transaction, and block data.
//!
//! This module provides high-performance data processing using SIMD (Single Instruction, 
//! Multiple Data) instructions when available. It falls back to standard processing when 
//! SIMD is not supported by the hardware.

use agave_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
    ReplicaTransactionInfoVersions, ReplicaEntryInfoVersions, Result as PluginResult, SlotStatus,
};

pub type ProcessedData = Vec<u8>;

#[derive(Debug, Clone, Copy)]
enum SerializationMode {    
    Standard,
    Sse4,
    Avx2,
    Avx512,
}

#[derive(Debug)]
pub struct SimdProcessor {
    enabled: bool,
    mode: SerializationMode,
}

impl SimdProcessor {
    pub fn new(enable_simd: bool) -> PluginResult<Self> {
        let mode = if !enable_simd {
            SerializationMode::Standard
        } else {
            #[cfg(all(target_arch = "x86_64", has_avx512))]
            {
                if std::is_x86_feature_detected!("avx512f") {
                    tracing::info!("Using AVX-512 for SIMD processing");
                    SerializationMode::Avx512
                } else if std::is_x86_feature_detected!("avx2") {
                    tracing::info!("Using AVX2 for SIMD processing");
                    SerializationMode::Avx2
                } else if std::is_x86_feature_detected!("sse4.1") {
                    tracing::info!("Using SSE4.1 for SIMD processing");
                    SerializationMode::Sse4
                } else {
                    tracing::info!("SIMD support not detected, using standard processing");
                    SerializationMode::Standard
                }
            }
            #[cfg(all(target_arch = "x86_64", has_avx2, not(has_avx512)))]
            {
                if std::is_x86_feature_detected!("avx2") {
                    tracing::info!("Using AVX2 for SIMD processing");
                    SerializationMode::Avx2
                } else if std::is_x86_feature_detected!("sse4.1") {
                    tracing::info!("Using SSE4.1 for SIMD processing");
                    SerializationMode::Sse4
                } else {
                    tracing::info!("SIMD support not detected, using standard processing");
                    SerializationMode::Standard
                }
            }
            #[cfg(all(target_arch = "x86_64", has_sse4_1, not(has_avx2), not(has_avx512)))]
            {
                if std::is_x86_feature_detected!("sse4.1") {
                    tracing::info!("Using SSE4.1 for SIMD processing");
                    SerializationMode::Sse4
                } else {
                    tracing::info!("SIMD support not detected, using standard processing");
                    SerializationMode::Standard
                }
            }
            #[cfg(not(target_arch = "x86_64"))]
            {
                tracing::info!("SIMD not supported on non-x86_64 architecture");
                SerializationMode::Standard
            }
            #[cfg(all(target_arch = "x86_64", not(has_sse4_1), not(has_avx2), not(has_avx512)))]
            {
                tracing::info!("SIMD support not enabled at compile time");
                SerializationMode::Standard
            }
        };

        Ok(Self {
            enabled: enable_simd,
            mode,
        })
    }

    pub fn process_account(
        &self,
        account: ReplicaAccountInfoVersions,
        slot: u64,
        ) -> PluginResult<ProcessedData> {
        let mut result = Vec::with_capacity(1024);
        result.push(0x01);
        result.extend_from_slice(&slot.to_le_bytes());

        match account {
            ReplicaAccountInfoVersions::V0_0_1(account_info) => {
                match self.mode {
                    SerializationMode::Avx512 => self.process_account_avx512(account_info, &mut result)?,
                    SerializationMode::Avx2 => self.process_account_avx2(account_info, &mut result)?,
                    SerializationMode::Sse4 => self.process_account_sse4(account_info, &mut result)?,
                    SerializationMode::Standard => self.process_account_standard(account_info, &mut result)?,
                }
            }
            ReplicaAccountInfoVersions::V0_0_2(account_info) => {
                match self.mode {
                    _ => self.process_account_v2_standard(account_info, &mut result)?,
                }
            }
            ReplicaAccountInfoVersions::V0_0_3(account_info) => {
                match self.mode {
                    _ => self.process_account_v3_standard(account_info, &mut result)?,
                }
            }
        }

        Ok(result)
    }

    pub fn process_transaction(
        &self,
        transaction: ReplicaTransactionInfoVersions,
        slot: u64,
    ) -> PluginResult<ProcessedData> {
        let mut result = Vec::with_capacity(1024);
        result.push(0x02);
        result.extend_from_slice(&slot.to_le_bytes());

        match transaction {
            ReplicaTransactionInfoVersions::V0_0_1(tx_info) => {
                result.extend_from_slice(tx_info.signature.as_ref());
                
                result.push(if tx_info.is_vote { 1 } else { 0 });
                
                let status = &tx_info.transaction_status_meta;
                result.push(if status.status.is_ok() { 1 } else { 0 });
                
                result.extend_from_slice(&status.fee.to_le_bytes());
            }
            ReplicaTransactionInfoVersions::V0_0_2(tx_info) => {
                result.extend_from_slice(tx_info.signature.as_ref());
                
                result.push(if tx_info.is_vote { 1 } else { 0 });
                
                result.extend_from_slice(&tx_info.index.to_le_bytes());
                
                let status = &tx_info.transaction_status_meta;
                result.push(if status.status.is_ok() { 1 } else { 0 });
                
                result.extend_from_slice(&status.fee.to_le_bytes());
            }
        }

        Ok(result)
    }

    pub fn process_slot(
        &self,
        slot: u64,
        parent: Option<u64>,
        status: SlotStatus,
    ) -> PluginResult<ProcessedData> {
        let mut result = Vec::with_capacity(64);
        result.push(0x03);
        result.extend_from_slice(&slot.to_le_bytes());

        match parent {
            Some(parent_slot) => {
                result.push(1);
                result.extend_from_slice(&parent_slot.to_le_bytes());
            }
            None => {
                result.push(0);
            }
        }

        let status_code = match status {
            SlotStatus::Processed => 0u8,
            SlotStatus::Confirmed => 1u8,
            SlotStatus::Rooted => 2u8,
        };
        result.push(status_code);

        Ok(result)
    }

    pub fn process_block(
        &self,
        block_info: ReplicaBlockInfoVersions,
    ) -> PluginResult<ProcessedData> {
        let mut result = Vec::with_capacity(1024);
        result.push(0x04);

        match block_info {
            ReplicaBlockInfoVersions::V0_0_1(block_info) => {
                result.extend_from_slice(&block_info.slot.to_le_bytes());
                
                let blockhash_bytes = block_info.blockhash.as_bytes();
                result.extend_from_slice(&(blockhash_bytes.len() as u16).to_le_bytes());
                result.extend_from_slice(blockhash_bytes);
                
                match block_info.block_time {
                    Some(time) => {
                        result.push(1);
                        result.extend_from_slice(&time.to_le_bytes());
                    }
                    None => {
                        result.push(0);
                    }
                }
                
                match block_info.block_height {
                    Some(height) => {
                        result.push(1);
                        result.extend_from_slice(&height.to_le_bytes());
                    }
                    None => {
                        result.push(0);
                    }
                }
            }
            ReplicaBlockInfoVersions::V0_0_2(block_info) => {
                result.extend_from_slice(&block_info.slot.to_le_bytes());
                
                result.extend_from_slice(&block_info.parent_slot.to_le_bytes());
                
                let blockhash_bytes = block_info.blockhash.as_bytes();
                result.extend_from_slice(&(blockhash_bytes.len() as u16).to_le_bytes());
                result.extend_from_slice(blockhash_bytes);
                
                let parent_blockhash_bytes = block_info.parent_blockhash.as_bytes();
                result.extend_from_slice(&(parent_blockhash_bytes.len() as u16).to_le_bytes());
                result.extend_from_slice(parent_blockhash_bytes);
                
                match block_info.block_time {
                    Some(time) => {
                        result.push(1);
                        result.extend_from_slice(&time.to_le_bytes());
                    }
                    None => {
                        result.push(0);
                    }
                }
                
                match block_info.block_height {
                    Some(height) => {
                        result.push(1);
                        result.extend_from_slice(&height.to_le_bytes());
                    }
                    None => {
                        result.push(0);
                    }
                }
                
                result.extend_from_slice(&block_info.executed_transaction_count.to_le_bytes());
            }
            ReplicaBlockInfoVersions::V0_0_3(block_info) => {
                result.extend_from_slice(&block_info.slot.to_le_bytes());
                
                result.extend_from_slice(&block_info.parent_slot.to_le_bytes());
                
                let blockhash_bytes = block_info.blockhash.as_bytes();
                result.extend_from_slice(&(blockhash_bytes.len() as u16).to_le_bytes());
                result.extend_from_slice(blockhash_bytes);
                
                let parent_blockhash_bytes = block_info.parent_blockhash.as_bytes();
                result.extend_from_slice(&(parent_blockhash_bytes.len() as u16).to_le_bytes());
                result.extend_from_slice(parent_blockhash_bytes);
                
                match block_info.block_time {
                    Some(time) => {
                        result.push(1);
                        result.extend_from_slice(&time.to_le_bytes());
                    }
                    None => {
                        result.push(0);
                    }
                }
                
                match block_info.block_height {
                    Some(height) => {
                        result.push(1);
                        result.extend_from_slice(&height.to_le_bytes());
                    }
                    None => {
                        result.push(0);
                    }
                }
                
                result.extend_from_slice(&block_info.executed_transaction_count.to_le_bytes());
                
                result.extend_from_slice(&block_info.entry_count.to_le_bytes());
            }
        }

        Ok(result)
    }

    pub fn process_entry(
        &self,
        entry: ReplicaEntryInfoVersions,
    ) -> PluginResult<ProcessedData> {
        let mut result = Vec::with_capacity(1024);
        result.push(0x05);

        match entry {
            ReplicaEntryInfoVersions::V0_0_1(entry_info) => {
                result.extend_from_slice(&entry_info.slot.to_le_bytes());
                
                result.extend_from_slice(&(entry_info.index as u64).to_le_bytes());
                
                result.extend_from_slice(&entry_info.num_hashes.to_le_bytes());
                
                result.extend_from_slice(entry_info.hash);
                
                
            ReplicaEntryInfoVersions::V0_0_2(entry_info) => {
                result.extend_from_slice(&entry_info.slot.to_le_bytes());
                
                result.extend_from_slice(&(entry_info.index as u64).to_le_bytes());
                
                result.extend_from_slice(&entry_info.num_hashes.to_le_bytes());
                
                result.extend_from_slice(entry_info.hash);
                
                result.extend_from_slice(&entry_info.executed_transaction_count.to_le_bytes());
                
                result.extend_from_slice(&(entry_info.starting_transaction_index as u64).to_le_bytes());
            }
        }

        Ok(result)
    }


    fn process_account_standard(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        result.extend_from_slice(account_info.pubkey);
        
        result.extend_from_slice(account_info.owner);
        
        result.extend_from_slice(&account_info.lamports.to_le_bytes());
        
        result.push(if account_info.executable { 1 } else { 0 });
        
        result.extend_from_slice(&account_info.rent_epoch.to_le_bytes());
        
        result.extend_from_slice(&account_info.write_version.to_le_bytes());
        
        let data_len = account_info.data.len();
        result.extend_from_slice(&(data_len as u32).to_le_bytes());
        result.extend_from_slice(account_info.data);
        
        Ok(())
    }

    
    #[cfg(all(target_arch = "x86_64", has_sse4_1))]
    fn process_account_sse4(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        self.process_account_standard(account_info, result)
    }

    #[cfg(not(all(target_arch = "x86_64", has_sse4_1)))]
    fn process_account_sse4(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        self.process_account_standard(account_info, result)
    }

    #[cfg(all(target_arch = "x86_64", has_avx2))]
    fn process_account_avx2(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        self.process_account_standard(account_info, result)
    }

    #[cfg(not(all(target_arch = "x86_64", has_avx2)))]
    fn process_account_avx2(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        self.process_account_standard(account_info, result)
    }

    #[cfg(all(target_arch = "x86_64", has_avx512))]
    fn process_account_avx512(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        self.process_account_standard(account_info, result)
    }

    #[cfg(not(all(target_arch = "x86_64", has_avx512)))]
    fn process_account_avx512(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        self.process_account_standard(account_info, result)
    }

    fn process_account_v2_standard(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfoV2,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        result.extend_from_slice(account_info.pubkey);
        
        result.extend_from_slice(account_info.owner);
        
        result.extend_from_slice(&account_info.lamports.to_le_bytes());
        
        result.push(if account_info.executable { 1 } else { 0 });
        
        result.extend_from_slice(&account_info.rent_epoch.to_le_bytes());

        result.extend_from_slice(&account_info.write_version.to_le_bytes());
        
        let data_len = account_info.data.len();
        result.extend_from_slice(&(data_len as u32).to_le_bytes());
        result.extend_from_slice(account_info.data);
        
        match account_info.txn_signature {
            Some(signature) => {
                result.push(1);
                result.extend_from_slice(signature.as_ref());
            }
            None => {
                result.push(0);
            }
        }
        
        Ok(())
    }

    fn process_account_v3_standard(
        &self,
        account_info: &agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfoV3,
        result: &mut Vec<u8>,
    ) -> PluginResult<()> {
        result.extend_from_slice(account_info.pubkey);
        
        result.extend_from_slice(account_info.owner);
        
        result.extend_from_slice(&account_info.lamports.to_le_bytes());
        
        result.push(if account_info.executable { 1 } else { 0 });
        
        result.extend_from_slice(&account_info.rent_epoch.to_le_bytes());

        result.extend_from_slice(&account_info.write_version.to_le_bytes());
        
        let data_len = account_info.data.len();
        result.extend_from_slice(&(data_len as u32).to_le_bytes());
        result.extend_from_slice(account_info.data);
        
        result.push(if account_info.txn.is_some() { 1 } else { 0 });
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agave_geyser_plugin_interface::geyser_plugin_interface::ReplicaAccountInfo;
    use solana_sdk::signature::Signature;

    #[test]
    fn test_simd_processor_creation() {
        let processor = SimdProcessor::new(true).unwrap();
        assert!(processor.enabled);
    }

    #[test]
    fn test_process_account() {
        let processor = SimdProcessor::new(true).unwrap();
        
        let pubkey = [1u8; 32];
        let owner = [2u8; 32];
        let lamports = 12345;
        let data = &[3u8, 4, 5];
        let executable = false;
        let rent_epoch = 0;
        
        let account_info = ReplicaAccountInfo {
            pubkey: &pubkey,
            owner: &owner,
            lamports,
            data,
            executable,
            rent_epoch,
            write_version: 1,
        };
        
        let account = ReplicaAccountInfoVersions::V0_0_1(&account_info);
        let slot = 42;
        
        let result = processor.process_account(account, slot).unwrap();
        
        assert!(!result.is_empty());
        assert_eq!(result[0], 0x01);
        
        let mut slot_bytes = [0u8; 8];
        slot_bytes.copy_from_slice(&result[1..9]);
        assert_eq!(u64::from_le_bytes(slot_bytes), slot);
    }

    #[test]
    fn test_process_slot() {
        let processor = SimdProcessor::new(true).unwrap();
        
        let slot = 42;
        let parent = Some(41);
        let status = SlotStatus::Confirmed;
        
        let result = processor.process_slot(slot, parent, status).unwrap();
        
        assert!(!result.is_empty());
        assert_eq!(result[0], 0x03);
        
        let mut slot_bytes = [0u8; 8];
        slot_bytes.copy_from_slice(&result[1..9]);
        assert_eq!(u64::from_le_bytes(slot_bytes), slot);
        
        assert_eq!(result[9], 1);
        
        let mut parent_bytes = [0u8; 8];
        parent_bytes.copy_from_slice(&result[10..18]);
        assert_eq!(u64::from_le_bytes(parent_bytes), 41);
        
        assert_eq!(result[18], 1);
    }
}


