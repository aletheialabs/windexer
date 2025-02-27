// crates/windexer-common/src/config/node.rs

use {
    serde::{Deserialize, Serialize},
    std::net::SocketAddr,
    crate::crypto::SerializableKeypair,
    std::any::Any,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Publisher,
    Relayer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherNodeConfig {
    pub node_id: String,
    pub node_type: NodeType,
    pub listen_addr: SocketAddr,
    pub rpc_addr: SocketAddr,
    pub bootstrap_peers: Vec<String>,
    pub data_dir: String,
    pub solana_rpc_url: String,
    pub geyser_plugin_config: Option<String>,
    pub keypair: SerializableKeypair,
    pub metrics_addr: Option<SocketAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerNodeConfig {
    pub node_id: String,
    pub node_type: NodeType,
    pub listen_addr: SocketAddr,
    pub rpc_addr: SocketAddr,
    pub bootstrap_peers: Vec<String>,
    pub data_dir: String,
    pub solana_rpc_url: String,
    pub geyser_plugin_config: Option<String>,
    pub keypair: SerializableKeypair,
    pub metrics_addr: Option<SocketAddr>,
}

pub trait NodeConfig: Any + std::fmt::Debug {
    fn get_node_type(&self) -> NodeType;
    fn get_config(&self) -> &dyn NodeConfig;
    fn get_keypair(&self) -> &SerializableKeypair;
    fn get_listen_addr(&self) -> &SocketAddr;
    fn get_bootstrap_peers(&self) -> &Vec<String>;
}

impl NodeConfig for PublisherNodeConfig {
    fn get_node_type(&self) -> NodeType {
        NodeType::Publisher
    }
    fn get_config(&self) -> &dyn NodeConfig {
        self
    }
    fn get_keypair(&self) -> &SerializableKeypair {
        &self.keypair
    }
    fn get_listen_addr(&self) -> &SocketAddr {
        &self.listen_addr
    }
    fn get_bootstrap_peers(&self) -> &Vec<String> {
        &self.bootstrap_peers
    }
}

impl NodeConfig for RelayerNodeConfig {
    fn get_node_type(&self) -> NodeType {
        NodeType::Relayer
    }
    fn get_config(&self) -> &dyn NodeConfig {
        self
    }
    fn get_keypair(&self) -> &SerializableKeypair {
        &self.keypair
    }
    fn get_listen_addr(&self) -> &SocketAddr {
        &self.listen_addr
    }
    fn get_bootstrap_peers(&self) -> &Vec<String> {
        &self.bootstrap_peers
    }
}

impl PublisherNodeConfig {
    pub fn new_local_publisher(
        node_id: impl Into<String> + std::fmt::Display,
        port: u16,
        rpc_port: u16,
        bootstrap_peers: Vec<String>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            node_type: NodeType::Publisher,
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

impl RelayerNodeConfig {
    pub fn new_local_relayer(
        node_id: impl Into<String> + std::fmt::Display,
        port: u16,
        rpc_port: u16,
        bootstrap_peers: Vec<String>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            node_type: NodeType::Relayer,
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