use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A mempool is a sequence of pending transactions that have not yet been included in a block.
#[derive(Debug, Serialize, Deserialize)]
pub struct MemPool {
    transaction_ids: HashSet<TransactionId>,
    transactions: Vec<BlockTransaction>,
    balance: HashMap<Address, u64>,
    prefix_hash: BlockHash,
}

impl MemPool {
    pub fn new(blockchain: &BlockChain) -> Self {
        MemPool {
            transaction_ids: HashSet::new(),
            transactions: vec![],
            balance: blockchain.balance().clone(),
            prefix_hash: blockchain.last_hash().clone(),
        }
    }

    /// The transactions in the mempool.
    pub fn transactions(&self) -> &Vec<BlockTransaction> {
        &self.transactions
    }

    /// The number of transactions in the mempool.
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn balance(&self) -> &HashMap<Address, u64> {
        &self.balance
    }

    pub fn balance_of(&mut self, address: Address) -> u64 {
        self.balance.get(&address).copied().unwrap_or(0)
    }

    pub fn balance_mut_of(&mut self, address: Address) -> &mut u64 {
        self.balance.entry(address).or_insert(0)
    }

    /// Add a transaction, checking whether it is valid.
    pub fn add_transaction(&mut self, transaction: BlockTransaction) -> Result<(), ()> {
        if transaction.prefix_hash != self.prefix_hash {
            warn!("Transaction {transaction} has a `prefix_hash` that is invalid for this mempool");
            return Err(());
        }
        if self.transaction_ids.contains(&transaction.id) {
            warn!("Transaction {transaction} is already in the mempool");
            return Err(());
        }
        if self.balance_of(transaction.info.sender) < transaction.info.amount {
            warn!(
                "Insufficient funds to transfer {} from {} to {}",
                transaction.info.amount, transaction.info.sender, transaction.info.receiver
            );
            return Err(());
        }
        self.transactions.push(transaction.clone());
        self.transaction_ids.insert(transaction.id);
        *self.balance_mut_of(transaction.info.sender) -= transaction.info.amount;
        *self.balance_mut_of(transaction.info.receiver) += transaction.info.amount;
        Ok(())
    }

    /// Reset the mempool to its initial state.
    pub fn reset(&mut self, blockchain: &BlockChain) {
        self.transactions.clear();
        self.transaction_ids.clear();
        self.balance = blockchain.balance().clone();
        self.prefix_hash = blockchain.last_hash().clone();
    }
}
