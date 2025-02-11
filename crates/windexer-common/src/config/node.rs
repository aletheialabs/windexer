// crates/windexer-common/src/config/node.rs

use {
    serde::{Deserialize, Serialize},
    std::net::SocketAddr,
    crate::crypto::SerializableKeypair,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,
    pub listen_addr: SocketAddr,
    pub rpc_addr: SocketAddr,
    pub bootstrap_peers: Vec<String>,
    pub data_dir: String,
    pub solana_rpc_url: String,
    pub geyser_plugin_config: Option<String>,
    pub keypair: SerializableKeypair,
    pub metrics_addr: Option<SocketAddr>,
}

impl NodeConfig {
    pub fn new_local(
        node_id: impl Into<String> + std::fmt::Display,
        port: u16,
        rpc_port: u16,
        bootstrap_peers: Vec<String>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            listen_addr: format!("127.0.0.1:{}", port).parse().unwrap(),
            rpc_addr: format!("127.0.0.1:{}", rpc_port).parse().unwrap(),
            bootstrap_peers,
            data_dir: format!("./data/node_{}", node_id),
            solana_rpc_url: "http://localhost:8899".to_string(),
            geyser_plugin_config: None,
            keypair: SerializableKeypair::default(),
            metrics_addr: None,
        }
    }
}