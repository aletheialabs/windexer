use solana_transaction_status::TransactionStatusMeta;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableTransactionMeta {
    pub status: Option<u64>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub inner_instructions: Option<Vec<SerializableInnerInstructions>>,
    pub log_messages: Option<Vec<String>>,
    pub pre_token_balances: Option<Vec<SerializableTokenBalance>>,
    pub post_token_balances: Option<Vec<SerializableTokenBalance>>,
    pub rewards: Option<Vec<SerializableReward>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableInnerInstructions {
    pub index: u8,
    pub instructions: Vec<SerializableInstruction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableInstruction {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableTokenBalance {
    pub account_index: u8,
    pub mint: String,
    pub ui_token_amount: SerializableUiTokenAmount,
    pub owner: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableUiTokenAmount {
    pub ui_amount: Option<f64>,
    pub decimals: u8,
    pub amount: String,
    pub ui_amount_string: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableReward {
    pub pubkey: String,
    pub lamports: i64,
    pub post_balance: u64,
    pub reward_type: Option<String>,
    pub commission: Option<u8>,
}

impl From<&TransactionStatusMeta> for SerializableTransactionMeta {
    fn from(meta: &TransactionStatusMeta) -> Self {
        let status = if meta.status.is_ok() {
            Some(0u64)
        } else {
            Some(1u64)
        };

        SerializableTransactionMeta {
            status,
            fee: meta.fee,
            pre_balances: meta.pre_balances.clone(),
            post_balances: meta.post_balances.clone(),
            inner_instructions: None,
            log_messages: meta.log_messages.clone(),
            pre_token_balances: None,
            post_token_balances: None,
            rewards: None,
        }
    }
} 