/// How many coins a new mined block gives to the miner.
pub const COINS_PER_MINED_BLOCK: u64 = 1000;

/// How many leading zero bits the hash of a mined block must have.
pub const MINING_DIFFICULTY: u32 = 20;

/// How many nonces to try in a row when mining, before yielding and reacting to the network.
pub const NODE_MINING_NONCE_STEP: u64 = 1000;
