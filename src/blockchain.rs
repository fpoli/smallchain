use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockChain {
    chain: Vec<BlockHash>,
    blocks: HashMap<BlockHash, Block>,
    balance: HashMap<Address, u64>,
}

impl BlockChain {
    pub fn new() -> Self {
        let genesis = Block::genesis();
        BlockChain {
            chain: vec![genesis.hash()],
            blocks: HashMap::from([(genesis.hash(), genesis)]),
            balance: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.chain.len()
    }

    pub fn contains(&self, block_hash: &BlockHash) -> bool {
        self.blocks.contains_key(block_hash)
    }

    pub fn get_block(&self, block_hash: &BlockHash) -> Option<&Block> {
        self.blocks.get(block_hash)
    }

    pub fn last_hash(&self) -> &BlockHash {
        let Some(block_hash) = self.chain.last() else {
            panic!("The blockchain of a node is empty");
        };
        block_hash
    }

    pub fn last_block(&self) -> &Block {
        let block_hash = self.last_hash();
        let Some(block) = self.blocks.get(block_hash) else {
            panic!("Cannot find block {block_hash}");
        };
        block
    }

    pub fn balance_mut(&mut self, address: Address) -> &mut u64 {
        self.balance.entry(address).or_insert(0)
    }

    pub fn balance(&self) -> &HashMap<Address, u64> {
        &self.balance
    }

    #[allow(dead_code)]
    pub fn balance_of(&self, address: Address) -> u64 {
        *self.balance.get(&address).unwrap_or(&0)
    }

    /// Appends a block to the blockchain. Returns an error if adding the block would make the
    /// blockchain invalid (e.g., invalid transactions, invalid block hash, etc.)
    pub fn append_block(&mut self, block: Block) -> Result<(), ()> {
        if &block.prefix_hash != self.last_hash() {
            warn!("Tried to append a block with an invalid prefix");
            return Err(());
        }
        if !block.is_valid_nonce() {
            warn!("Tried to append an invalid block");
            return Err(());
        }

        // Check that the ids of the transactions are unique
        let mut transaction_ids = HashSet::new();
        for t in &block.transactions {
            if !transaction_ids.insert(t.id) {
                warn!("Tried to append a block with duplicate transaction ids");
                return Err(());
            }
        }

        // Check and update the balance
        for t in &block.transactions {
            if t.prefix_hash != block.prefix_hash {
                warn!("Tried to append a block with a transaction with an invalid `prefix_hash`");
                return Err(());
            }
            if *self.balance_mut(t.info.sender) < t.info.amount {
                warn!("Tried to append a block with invalid transactions");
                return Err(());
            }
            *self.balance_mut(t.info.sender) -= t.info.amount;
            *self.balance_mut(t.info.receiver) += t.info.amount;
        }
        *self.balance_mut(block.miner) += COINS_PER_MINED_BLOCK;

        // Add the block to the blockchain
        let block_hash = block.hash();
        self.chain.push(block_hash.clone());
        self.blocks.insert(block_hash, block.clone());

        Ok(())
    }

    /// Pops a block from the blockchain
    pub fn pop_block(&mut self) -> Option<Block> {
        if self.last_block().is_genesis() {
            return None;
        }

        let block_hash = self.chain.pop().unwrap();
        let block = self.blocks.remove(&block_hash).unwrap();

        *self.balance_mut(block.miner) -= COINS_PER_MINED_BLOCK;
        for t in &block.transactions {
            *self.balance_mut(t.info.sender) += t.info.amount;
            *self.balance_mut(t.info.receiver) -= t.info.amount;
        }

        Some(block)
    }

    /// Pops blocks until the block with the given hash is the latest
    pub fn pop_until(&mut self, block_hash: &BlockHash) {
        while self.last_hash() != block_hash {
            self.pop_block().unwrap();
        }
    }

    /// Appends a list of block to the blockchain. Returns an error if adding the block would make
    /// the blockchain invalid (e.g., invalid transactions, invalid block hash, etc.)
    pub fn append_blocks(&mut self, blocks: impl IntoIterator<Item = Block>) -> Result<(), ()> {
        for block in blocks {
            self.append_block(block)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mine_three_blocks() {
        let mut blockchain = BlockChain::new();
        let miner = Address::new(1);
        for _ in 0..3 {
            let new_block =
                attempt_mining_block(blockchain.last_hash().clone(), miner, vec![], 0..=u64::MAX)
                    .unwrap();
            blockchain.append_block(new_block).unwrap();
        }
        assert!(blockchain.len() == 4);
        assert!(blockchain.balance().len() == 1);
        assert!(blockchain.balance_of(miner) == 3 * COINS_PER_MINED_BLOCK);
    }
}
