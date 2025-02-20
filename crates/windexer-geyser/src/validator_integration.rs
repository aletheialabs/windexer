// crates/windexer-geyser/src/validator_integration.rs
use {
    crossbeam_channel::{Receiver, Sender, bounded, unbounded},
    solana_runtime::{
        accounts_background_service::AbsRequest,
        bank_forks::BankForks,
        commitment::BlockCommitmentCache,
        gepfired::Gepfired,
        pipeline::{
            PipelineStage, ProcessingCallback, TransactionBatch, 
            TransactionBatchProcessingOptions, VerifiedTransaction
        },
    },
    solana_sdk::{clock::Slot, transaction::SanitizedTransaction},
    std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        time::Duration,
    },
    thiserror::Error,
    windexer_common::types::{TideBatch, TideHeader},
};

#[derive(Error, Debug)]
pub enum ValidatorIntegrationError {
    #[error("Pipeline stage registration failed")]
    RegistrationFailure,
    #[error("Bank forks access error")]
    BankAccessError,
    #[error("Transaction validation failed")]
    ValidationError,
    #[error("Channel communication error")]
    ChannelError,
}

/// Taps into specific pipeline stages
pub struct ValidatorTap {
    stage: PipelineStage,
    sender: Sender<TideBatch>,
    running: Arc<AtomicBool>,
    bank_forks: Arc<BankForks>,
    block_commitment_cache: Arc<BlockCommitmentCache>,
}

impl ValidatorTap {
    /// Create new pipeline tap with associated receiver
    pub fn new(
        stage: PipelineStage,
        bank_forks: Arc<BankForks>,
        block_commitment_cache: Arc<BlockCommitmentCache>,
        buffer_size: usize,
    ) -> (Self, Receiver<TideBatch>) {
        let (tx, rx) = bounded(buffer_size);
        (
            Self {
                stage,
                sender: tx,
                running: Arc::new(AtomicBool::new(false)),
                bank_forks,
                block_commitment_cache,
            },
            rx,
        )
    }

    /// Start listening to pipeline events
    pub fn install(&self) -> Result<(), ValidatorIntegrationError> {
        let callback = self.create_processing_callback();
        let options = TransactionBatchProcessingOptions {
            enable_cpi_recording: false,
            transaction_status_sender: None,
            enable_rpc_transaction_history: false,
            enable_cost_model: false,
        };

        self.stage.register_processor(
            Box::new(callback),
            options,
            self.bank_forks.clone(),
            self.block_commitment_cache.clone(),
        )
        .map_err(|_| ValidatorIntegrationError::RegistrationFailure)?;

        self.running.store(true, Ordering::Release);
        Ok(())
    }

    /// Create processing callback that taps into pipeline
    fn create_processing_callback(&self) -> ProcessingCallback {
        let sender = self.sender.clone();
        let running = self.running.clone();

        Box::new(
            move |bank: &Arc<BankForks>,
                  batch: &TransactionBatch,
                  _gepfired: &Gepfired| -> Result<(), AbsRequest> {
                if !running.load(Ordering::Acquire) {
                    return Ok(());
                }

                let slot = bank.read().unwrap().root();
                let txs = batch.transactions()
                    .iter()
                    .filter_map(|tx| {
                        SanitizedTransaction::try_create(
                            tx.clone(),
                            bank.read().unwrap().hash(),
                            bank.read().unwrap().fee_structure().clone(),
                            bank.read().unwrap().rent_collector().rent,
                            true,
                        )
                        .ok()
                    })
                    .collect::<Vec<_>>();

                let batch = TideBatch {
                    header: TideHeader {
                        slot,
                        parent_slot: slot.saturating_sub(1),
                        timestamp: bank.read().unwrap().clock().unix_timestamp,
                        chain_id: bank.read().unwrap().chain_id(),
                    },
                    transactions: txs,
                };

                sender.send(batch).map_err(|_| AbsRequest::default())?;
                Ok(())
            },
        )
    }

    /// Stop listening to pipeline events
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Release);
    }
}

/// Manages multiple pipeline taps
pub struct TideHook {
    taps: Vec<ValidatorTap>,
    receivers: Vec<Receiver<TideBatch>>,
    current_slot: Slot,
}

impl TideHook {
    /// Create new hook with specified pipeline stages
    pub fn new(
        stages: Vec<PipelineStage>,
        bank_forks: Arc<BankForks>,
        block_commitment_cache: Arc<BlockCommitmentCache>,
    ) -> Self {
        let (taps, receivers) = stages
            .into_iter()
            .map(|stage| {
                ValidatorTap::new(
                    stage,
                    bank_forks.clone(),
                    block_commitment_cache.clone(),
                    100_000,
                )
            })
            .unzip();

        Self {
            taps,
            receivers,
            current_slot: 0,
        }
    }

    /// Install all registered taps
    pub fn install_all(&mut self) -> Result<(), ValidatorIntegrationError> {
        for tap in &self.taps {
            tap.install()?;
        }
        Ok(())
    }

    /// Get next batch of transactions in slot order
    pub fn poll_batch(&mut self, timeout: Duration) -> Option<TideBatch> {
        crossbeam_channel::select! {
            recv(self.receivers[0]) -> batch => batch.ok(),
            recv(self.receivers[1]) -> batch => batch.ok(),
            default(timeout) => None
        }
    }

    /// Get current highest processed slot
    pub fn current_slot(&self) -> Slot {
        self.current_slot
    }

    /// Shutdown all taps
    pub fn shutdown(self) {
        for tap in self.taps {
            tap.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_runtime::{
        bank::Bank,
        genesis_utils::{create_genesis_config, GenesisConfigInfo},
    };

    fn setup_test_environment() -> (Arc<BankForks>, Arc<BlockCommitmentCache>) {
        let GenesisConfigInfo { genesis_config, .. } = create_genesis_config(100);
        let bank = Bank::new_for_tests(&genesis_config);
        let bank_forks = Arc::new(BankForks::new(bank));
        let block_commitment_cache = Arc::new(BlockCommitmentCache::default());
        (bank_forks, block_commitment_cache)
    }

    #[test]
    fn test_pipeline_tap_creation() {
        let (bank_forks, block_commitment_cache) = setup_test_environment();
        let (tap, _) = ValidatorTap::new(
            PipelineStage::TPUReceive,
            bank_forks,
            block_commitment_cache,
            100,
        );
        assert!(tap.install().is_ok());
    }

    #[test]
    fn test_tide_hook_operation() {
        let (bank_forks, block_commitment_cache) = setup_test_environment();
        let mut hook = TideHook::new(
            vec![PipelineStage::TPUReceive, PipelineStage::TVUVote],
            bank_forks,
            block_commitment_cache,
        );
        
        assert!(hook.install_all().is_ok());
        assert!(hook.poll_batch(Duration::from_millis(100)).is_none());
        hook.shutdown();
    }
}
