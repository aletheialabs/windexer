// crates/windexer-common/src/config/node.rs

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,
    pub listen_addr: SocketAddr,
    pub rpc_addr: SocketAddr,
    pub bootstrap_peers: Vec<String>,
    pub data_dir: String,
    pub solana_rpc_url: String,
    pub geyser_plugin_config: Option<String>,
}

impl NodeConfig {
    pub fn new_local(
        node_id: impl Into<String>,
        port: u16,
        rpc_port: u16,
        bootstrap_peers: Vec<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            listen_addr: format!("127.0.0.1:{}", port).parse().unwrap(),
            rpc_addr: format!("127.0.0.1:{}", rpc_port).parse().unwrap(),
            bootstrap_peers,
            data_dir: format!("./data/node_{}", node_id),
            solana_rpc_url: "http://localhost:8899".to_string(),
            geyser_plugin_config: None,
        }
    }
}