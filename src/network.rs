use crate::prelude::*;
use tokio::sync::OnceCell;

/// The network interface that a node uses to communicate with other nodes.
pub trait NetworkInterface {
    /// Broadcast a new mined block to all other nodes.
    fn broadcast_block(
        &self,
        block: &Block,
        blockchain_length: usize,
        source: Address,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Broadcast a new pending transaction to all nodes.
    fn broadcast_transaction(
        &self,
        transaction: &BlockTransaction,
        source: Address,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Query a block from a specific node.
    fn query_block(
        &self,
        block_hash: &BlockHash,
        destination: Address,
    ) -> impl std::future::Future<Output = Option<Block>> + Send;
}

/// Get the singleton of the network interface.
pub async fn network() -> &'static impl NetworkInterface {
    NETWORK.get_or_init(|| async { FakeNetwork::new() }).await
}

static NETWORK: OnceCell<FakeNetwork> = OnceCell::const_new();

/// A fake network that simulates the communication between nodes.
struct FakeNetwork;

impl FakeNetwork {
    pub fn new() -> Self {
        FakeNetwork
    }
}

impl NetworkInterface for FakeNetwork {
    async fn broadcast_block(&self, block: &Block, blockchain_length: usize, source: Address) {
        debug!("Node {source} broadcasts block {block}");
        let mut addresses = world().await.get_node_addresses().await;
        for address in addresses.drain(..) {
            if address == source {
                continue;
            }
            let cloned_block = block.clone();
            tokio::spawn(async move {
                let Some(node) = world().await.get_node(address).await else {
                    warn!("Cannot find node {address} to broadcast block {cloned_block}");
                    return;
                };
                node.write()
                    .await
                    .receive_new_block(cloned_block, blockchain_length, source)
                    .await;
            });
        }
    }

    async fn broadcast_transaction(&self, transaction: &BlockTransaction, source: Address) {
        debug!("Node {source} broadcasts transaction {transaction}");
        let mut addresses = world().await.get_node_addresses().await;
        for address in addresses.drain(..) {
            if address == source {
                continue;
            }
            let cloned_transaction = transaction.clone();
            tokio::spawn(async move {
                let Some(node) = world().await.get_node(address).await else {
                    warn!(
                        "Cannot find node {address} to broadcast transaction {cloned_transaction}"
                    );
                    return;
                };
                node.write().await.add_transaction(cloned_transaction);
            });
        }
    }

    async fn query_block(&self, block_hash: &BlockHash, destination: Address) -> Option<Block> {
        debug!("Querying block {block_hash} from {destination}");
        let Some(node) = world().await.get_node(destination).await else {
            warn!("Cannot find node {destination} to query block {block_hash}");
            return None;
        };
        let readable_node = node.read().await;
        let Some(block) = readable_node.get_block(block_hash) else {
            warn!("Node {destination} does not have block {block_hash}");
            return None;
        };
        Some(block)
    }
}
