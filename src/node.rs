use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Run a node in the blockchain network. This function will run until the node is stopped.
pub async fn run_node(node: Arc<RwLock<Node>>) {
    loop {
        if !node.read().await.alive {
            break;
        }

        let mut writable_node = node.write().await;
        writable_node.achieve_consensus().await;
        writable_node.mining().await;

        // It's important to release all lock before yielding, to avoid deadlocks.
        drop(writable_node);

        // Cooperative preemption.
        tokio::task::yield_now().await;
    }
}

/// A node in the blockchain network.
#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    /// Whether the node should continue running.
    alive: bool,
    /// The address of the node.
    address: Address,
    /// The blockchain managed by the node.
    blockchain: BlockChain,
    /// The nonce to start from for the next mining attempt.
    next_nonce: u64,
    /// The pensind transactions accepted by the node.
    mempool: MemPool,
    /// A better blockchain proposed by the network.
    better_blockchain: Option<BetterBlockChain>,
}

impl Node {
    pub fn new() -> Self {
        let blockchain = BlockChain::new();
        let mempool = MemPool::new(&blockchain);
        Node {
            alive: true,
            address: Address::new_random(),
            blockchain,
            next_nonce: 0,
            mempool,
            better_blockchain: None,
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    /// Stop the node.
    pub fn stop(&mut self) {
        self.alive = false;
    }

    pub fn get_block(&self, block: &BlockHash) -> Option<Block> {
        self.blockchain.get_block(block).cloned()
    }

    pub fn blockchain(&self) -> &BlockChain {
        &self.blockchain
    }

    pub fn mempool(&self) -> &MemPool {
        &self.mempool
    }

    /// Attempt to mine a new block. If successful, the block is appended to the local blockchain
    /// and broadcasted to the network.
    async fn mining(&mut self) {
        let last_nonce = self.next_nonce + NODE_MINING_NONCE_STEP;
        let opt_block = attempt_mining_block(
            self.blockchain.last_hash().clone(),
            self.address,
            // TODO: Cloning these transactions is not necessary to compute the hash of a block.
            self.mempool.transactions().clone(),
            self.next_nonce..last_nonce,
        );
        if let Some(block) = opt_block {
            info!("Node {self}: Mined block {block}");
            if self.blockchain.append_block(block.clone()).is_err() {
                unreachable!("Node {self}: The mined block is invalid");
            }
            self.next_nonce = 0;
            self.mempool.reset(&self.blockchain);

            network()
                .await
                .broadcast_block(&block, self.blockchain.len(), self.address)
                .await;
        } else {
            self.next_nonce = last_nonce;
        }
    }

    /// Receive a new block from the network, without checking its validity.
    /// If the received blockchain is better than the local one, it is stored for later consensus.
    pub async fn receive_new_block(
        &mut self,
        block: Block,
        blockchain_length: usize,
        source: Address,
    ) {
        if blockchain_length <= self.blockchain.len() {
            return;
        }

        // Check if self.better_blockchain is already better than the received one
        if let Some(better_blockchain) = self.better_blockchain.as_ref() {
            if better_blockchain.length >= blockchain_length {
                debug!(
                    "Node {self}: Ignoring a new blockchain of length {blockchain_length} from \
                    {source} because we already have a better one of length {} from {}",
                    better_blockchain.length, better_blockchain.source
                );
                return;
            }
        }

        self.better_blockchain = Some(BetterBlockChain {
            length: blockchain_length,
            last_block: block.clone(),
            source,
        });
    }

    /// Switch to a better (i.e., longer) blockchain if one is available.
    /// Invalid blockchains are logged and discarded.
    async fn achieve_consensus(&mut self) {
        let Some(better_blockchain) = self.better_blockchain.take() else {
            return;
        };

        if better_blockchain.length <= self.blockchain.len() {
            return;
        }

        let source = better_blockchain.source;
        let mut last_common_hash = better_blockchain.last_block.hash();
        let mut new_blocks = vec![];
        if !self.blockchain.contains(&last_common_hash) {
            last_common_hash = better_blockchain.last_block.prefix_hash.clone();
            new_blocks.push(better_blockchain.last_block);
            while !self.blockchain.contains(&last_common_hash) {
                let block = network().await.query_block(&last_common_hash, source).await;
                if let Some(block) = block {
                    last_common_hash = block.prefix_hash.clone();
                    new_blocks.push(block);
                } else {
                    error!(
                        "Node {self}: Failed to fetch block {last_common_hash} from the network"
                    );
                    return;
                }
            }
        }

        // Check if the proposed blockchain is valid.
        // TODO: It is possible to do this more efficiently, without cloning and traversing the
        // full blockchain, by just checking the difference between the two blockchains.
        let mut new_blockchain = self.blockchain.clone();
        new_blockchain.pop_until(&last_common_hash);
        if new_blockchain
            .append_blocks(new_blocks.into_iter().rev())
            .is_err()
        {
            error!("Node {self}: The proposed better blockchain is invalid");
            return;
        }

        if new_blockchain.len() != better_blockchain.length {
            error!(
                "Node {self}: The proposed better blockchain has an invalid length ({} != {})",
                new_blockchain.len(),
                better_blockchain.length
            );
            return;
        }

        info!(
            "Node {self}: Accepting a new blockchain of length {} from {source} (old length: {})",
            new_blockchain.len(),
            self.blockchain.len()
        );
        self.blockchain = new_blockchain;
        self.next_nonce = 0;
        self.mempool.reset(&self.blockchain);
    }

    /// Add a transaction send from a client to the mempool and broadcast it to the network.
    /// Returns an error if the transaction is invalid.
    pub async fn add_client_transaction(&mut self, transaction: Transaction) -> Result<(), ()> {
        let block_transaction = BlockTransaction::new_with_random_id(
            self.blockchain.last_hash().clone(),
            transaction.clone(),
        );
        info!("Node {self}: Received transaction {block_transaction} from a client");
        if self
            .mempool
            .add_transaction(block_transaction.clone())
            .is_err()
        {
            error!("Node {self}: Rejecting transaction {block_transaction}");
            return Err(());
        };
        error!("Node {self}: Accepted transaction {block_transaction}");
        network()
            .await
            .broadcast_transaction(&block_transaction, self.address)
            .await;
        Ok(())
    }

    /// Add a transaction received from the network to the mempool.
    /// Invalid transactions are logged and discarded.
    pub fn add_transaction(&mut self, transaction: BlockTransaction) {
        info!("Node {self}: Received transaction {transaction} from the network");
        if self.mempool.add_transaction(transaction.clone()).is_err() {
            warn!("Node {self}: Ignoring invalid transaction {transaction}");
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.address)
    }
}

/// A potentially better blockchain received from the network.
#[derive(Debug, Serialize, Deserialize)]
struct BetterBlockChain {
    /// The length of the proposed blockchain.
    length: usize,
    /// The last block of the proposed blockchain.
    last_block: Block,
    /// The address of the node that proposed the blockchain.
    source: Address,
}
