use crate::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// In the blockchain, each address is associated to a certain amount of coins. Transactions can
/// update this amount. Each node in the network is also identified by an address. Mining a block
/// rewards the address of the miner with a certain amount of coins.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Address(u64);

impl Address {
    pub fn new_random() -> Self {
        Address(rand::thread_rng().gen())
    }

    /// Create an address with a specific identifier. Only used for determinism in testing.
    #[cfg(test)]
    pub(crate) fn new(id: u64) -> Self {
        Address(id)
    }

    pub fn from_str(s: &str) -> Result<Self, std::num::ParseIntError> {
        Ok(Address(s.parse::<u64>()?))
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "@{}", self.0)
    }
}

/// The identifier of the transaction. This needs to be unique at least among the transactions that
/// are part of the same block.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TransactionId(u64);

impl TransactionId {
    pub fn new_random() -> Self {
        TransactionId(rand::thread_rng().gen())
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "${}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Transaction {
    /// The address of the sender of the transaction.
    pub sender: Address,
    /// The address of the receiver of the transaction.
    pub receiver: Address,
    /// The amount of transferred coins.
    pub amount: u64,
}

impl Transaction {
    pub fn new(sender: Address, receiver: Address, amount: u64) -> Self {
        Transaction {
            sender,
            receiver,
            amount,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BlockTransaction {
    /// The identifier of the transaction.
    pub id: TransactionId,
    /// The hash of the block preceeding the one that contains this transaction. This is used to
    /// efficiently check that the transaction is not executed twice in the blockchain.
    pub prefix_hash: BlockHash,
    /// Information about the sender, receiver, and amount of the transaction.
    pub info: Transaction,
}

impl BlockTransaction {
    pub fn new_with_random_id(prefix_hash: BlockHash, info: Transaction) -> Self {
        BlockTransaction {
            id: TransactionId::new_random(),
            prefix_hash,
            info,
        }
    }
}

impl std::fmt::Display for BlockTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// The hash of a block. This is used to uniquely identify a block in the blockchain.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
// TODO: it would be much more idiomatic to make this Copy. Vec<u8> is an overkill, because we
// always know the number of bits.
pub struct BlockHash(Vec<u8>);

impl BlockHash {
    pub fn inner(&self) -> &[u8] {
        &self.0
    }

    pub fn from_str(s: &str) -> Result<Self, std::num::ParseIntError> {
        let mut bytes = vec![];
        for i in 0..s.len() / 2 {
            let byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16)?;
            bytes.push(byte);
        }
        Ok(BlockHash(bytes))
    }

    /// Count the number of leading zero **bits** in the hash.
    pub fn leading_zero_bits(&self) -> u32 {
        let mut leading_zeros = 0;
        for &value in self.inner().iter() {
            debug_assert!((value == 0) == (value.leading_zeros() == 8));
            if value == 0 {
                leading_zeros += 8;
            } else {
                leading_zeros += value.leading_zeros();
                break;
            }
        }
        leading_zeros
    }
}

impl std::fmt::Display for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#")?;
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

/// A block in the blockchain.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Block {
    /// The transactions in the block. They must all have the same `prefix_hash` of this block.
    pub transactions: Vec<BlockTransaction>,
    /// The hash of the block preceeding this one in the blockchain.
    pub prefix_hash: BlockHash,
    /// The address to which the mining reward is given.
    pub miner: Address,
    /// The nonce used to mine the block.
    pub nonce: u64,
}

impl Block {
    pub fn genesis() -> Self {
        Block {
            transactions: vec![],
            prefix_hash: BlockHash(vec![]),
            miner: Address(0),
            nonce: 0,
        }
    }

    pub fn is_genesis(&self) -> bool {
        self.prefix_hash.inner().is_empty()
    }

    pub fn new(
        transactions: Vec<BlockTransaction>,
        prefix_hash: BlockHash,
        miner: Address,
        nonce: u64,
    ) -> Self {
        Block {
            transactions,
            prefix_hash,
            miner,
            nonce,
        }
    }

    /// Compute the hash of the block.
    pub fn hash(&self) -> BlockHash {
        let mut hasher = Sha256::new();
        let serialized: Vec<u8> = bincode::serialize(self).expect("Failed to serialize a block");
        hasher.update(serialized);
        let hash = hasher.finalize();
        BlockHash(hash.to_vec())
    }

    /// Check if the nonce of the block is valid. Note: this does not check whether the transactions
    /// in the block are valid.
    pub fn is_valid_nonce(&self) -> bool {
        self.hash().leading_zero_bits() >= MINING_DIFFICULTY
    }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.hash())
    }
}

/// Attempt to mine a block using the nounces generated by an iterator.
pub fn attempt_mining_block(
    prefix_hash: BlockHash,
    miner: Address,
    transactions: Vec<BlockTransaction>,
    nonces: impl Iterator<Item = u64>,
) -> Option<Block> {
    let mut new_block = Block::new(transactions, prefix_hash, miner, 0);
    for nonce in nonces {
        new_block.nonce = nonce;
        if new_block.is_valid_nonce() {
            return Some(new_block);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mine_three_blocks() {
        let mut block = Block::genesis();
        let miner = Address::new(1);
        for _ in 0..3 {
            block = attempt_mining_block(block.hash(), miner, vec![], 0..=u64::MAX).unwrap();
        }
    }

    #[test]
    fn leading_zero_bits() {
        let mut block = Block::genesis();
        let miner = Address::new(2);
        block = attempt_mining_block(block.hash(), miner, vec![], 0..=u64::MAX).unwrap();
        assert!(block.hash().leading_zero_bits() >= MINING_DIFFICULTY);
    }
}
