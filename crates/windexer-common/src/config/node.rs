// crates/windexer-common/src/config/node.rs

use {
    serde::{Deserialize, Serialize},
    std::net::SocketAddr,
    crate::crypto::SerializableKeypair,
    std::any::Any,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    PUBLISHER,
    RELAYER,
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

pub trait NodeConfig: Any {
    fn get_node_type(&self) -> NodeType;
    fn get_listen_addr(&self) -> &SocketAddr;
    fn get_bootstrap_peers(&self) -> &Vec<String>;
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

impl NodeConfig for PublisherNodeConfig {
    fn get_node_type(&self) -> NodeType {
        NodeType::PUBLISHER
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
        NodeType::RELAYER
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
            node_type: NodeType::PUBLISHER,
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
            node_type: NodeType::RELAYER,
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