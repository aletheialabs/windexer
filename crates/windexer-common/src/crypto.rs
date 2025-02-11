use {
    anyhow,
    bs58,
    serde::{Deserialize, Serialize},
    solana_sdk::signer::keypair::Keypair,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableKeypair(String);

impl SerializableKeypair {
    pub fn new(keypair: &Keypair) -> Self {
        Self(bs58::encode(keypair.to_bytes()).into_string())
    }

    pub fn to_keypair(&self) -> anyhow::Result<Keypair> {
        let bytes = bs58::decode(&self.0)
            .into_vec()
            .map_err(|e| anyhow::anyhow!("Failed to decode keypair: {}", e))?;
        
        Keypair::from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("Invalid keypair bytes: {}", e))
    }
}

impl Default for SerializableKeypair {
    fn default() -> Self {
        Self::new(&Keypair::new())
    }
} 