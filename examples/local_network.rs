// examples/local_network.rs

use anyhow::Result;
use tokio;
use windexer_common::config::NodeConfig;
use windexer_network::Node;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Define node configurations
    let node_configs = vec![
        NodeConfig::new_local("node1", 9000, 8000, vec![]),
        NodeConfig::new_local("node2", 9001, 8001, vec!["127.0.0.1:9000".to_string()]),
        NodeConfig::new_local("node3", 9002, 8002, vec!["127.0.0.1:9000".to_string()]),
    ];

    // Launch nodes
    let mut handles = vec![];
    
    for config in node_configs {
        let handle = tokio::spawn(async move {
            let node = Node::new(config).await?;
            node.start().await
        });
        handles.push(handle);
    }

    // Wait for all nodes
    for handle in handles {
        handle.await??;
    }

    Ok(())
}