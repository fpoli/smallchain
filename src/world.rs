use crate::prelude::*;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::OnceCell;
use tokio::sync::RwLock;

/// The world that contains all nodes of the blockchain network.
pub struct World {
    /// The nodes in the blockchain network.
    /// The outer `RwLock` is only write-locked when adding or removing nodes.
    /// The inner `RwLock` is periodically write-locked when a node is running.
    nodes: RwLock<HashMap<Address, Arc<RwLock<Node>>>>,
}

static WORLD: OnceCell<World> = OnceCell::const_new();

/// Get the singleton of the world.
pub async fn world() -> &'static World {
    WORLD.get_or_init(|| async { World::new() }).await
}

impl World {
    fn new() -> Self {
        World {
            nodes: RwLock::new(HashMap::new()),
        }
    }

    /// Get a node by its address.
    pub async fn get_node(&self, address: Address) -> Option<Arc<RwLock<Node>>> {
        self.nodes.read().await.get(&address).cloned()
    }

    /// Get the addresses of all nodes.
    pub async fn get_node_addresses(&self) -> Vec<Address> {
        self.nodes.read().await.keys().cloned().collect()
    }

    /// Add a new node to the world, starting its execution.
    pub async fn add_node(&self) -> Address {
        let node = Node::new();
        info!("Create node {node}");
        let address = node.address();
        let node_arc = Arc::new(RwLock::new(node));
        self.nodes.write().await.insert(address, node_arc.clone());
        tokio::spawn(run_node(node_arc));
        address
    }

    /// Remove a node from the world, stopping its execution.
    pub async fn delete_node(&self, address: Address) {
        info!("Remove node {address}");
        let Some(node) = self.nodes.write().await.remove(&address) else {
            warn!("Cannot remove inexistent node {address}");
            return;
        };
        node.write().await.stop();
    }
}
