use sha2::{Digest, Sha256};
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
};

/// Hash a message using SHA-256
pub fn hash_message(message: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(message);
    hasher.finalize().to_vec()
}

/// Sign a message using a keypair
pub fn sign_message(keypair: &Keypair, message: &[u8]) -> Signature {
    keypair.sign_message(message)
}

/// Verify a message signature
pub fn verify_signature(pubkey: &[u8], message: &[u8], signature: &[u8]) -> bool {
    if let (Ok(pubkey), Ok(signature)) = (
        solana_sdk::pubkey::Pubkey::try_from(pubkey),
        solana_sdk::signature::Signature::try_from(signature),
    ) {
        signature.verify(pubkey.as_ref(), message)
    } else {
        false
    }
}

/// Generate a new random keypair
pub fn generate_keypair() -> Keypair {
    Keypair::new()
}