use {
    libp2p::identity,
    solana_sdk::signer::keypair::Keypair as SolanaKeypair,
};

pub fn convert_keypair(solana_keypair: &SolanaKeypair) -> identity::Keypair {
    let secret = solana_keypair.to_bytes();
    identity::Keypair::ed25519_from_bytes(secret).expect("Valid keypair conversion")
} 