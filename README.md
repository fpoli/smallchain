# SmallChain

A Tokio-based simulation of a simple blockchain, with a REST API to interact with it.

## Description

The blockchain of this simulation is a (drastically) simpler version of the Bitcoin blockchain. In the simulation, the nodes of a network are modeled as long-running Tokio tasks, which periodically mine new blocks and react to each other. A REST API is provided to add and remove nodes, send transactions, and query the state of the network.

Features:
* Each node keeps a list of pending transactions, which are included in the next block that it mines.
* When a new block is mined, it is advertised to all other nodes.
* When a node accepts a new (pending) transaction from a client, it advertises it to all other nodes.
* When a node observes that there is a longer blockchain in the network, it updates its local blockchain to the longest one, after checking that the new blockchain is valid.

Notable simplifications, compared to a real-world blockchain such as Bitcoin:
* The nodes do not make real network communications; they only send messages to each other through an asynchronous trait interface.
* The transactions are not authenticated. The REST API freely allows sending transactions between addresses.
* The implementation checks the validity of the blockchain and transactions, but it does not attempt to prevent DDOS attacks. For example, a node can block the network by continuously advertising an improbably long, randomly generated, blockchain.
* The transactions update the amount of coins associated with an address. This is different from Bitcoin, where the transactions have to fully move the coins from several input addresses to several new output addresses.
* To efficiently check that a transaction is not used twice in the blockchain, each transaction is parameterized by the hash of the last block on the local blockchain (i.e., the block preceding the one in which the transaction will be stored) of the first node receiving the transaction. Because of this, when an orphan block is removed from the local blockchain, the transactions that it contains are dropped instead of being re-added to the pending transactions of the node.

## Usage

```text
$ smallchain --help
Simulator of a simple blockchain

Usage: smallchain [OPTIONS]

Options:
  -p, --port <PORT>  The port on which the server will listen
  -d, --demo         Enable the demo mode
  -h, --help         Print help
```

## REST API

* `GET  /`: Check that the server is running.
* `GET  /nodes`: Get a JSON list of the addresses of the nodes in the network.
* `POST /node`: Create a new node and return its address.
* `DEL  /node/{address}`: Removes a node from the network.
* `GET  /node/{address}`: Display information about a node.
* `GET  /node/{address}/block/{hash}`: Show a block of the local blockchain of a node.
* `GET  /node/{address}/blockchain_balance`: Get the final balance of the local blockchain of a node.
* `GET  /node/{address}/mempool_balance`: Get the final balance, including pending transactions, of a node.
* `POST /node/{address}/send/from/{from_address}/to/{to_address}/amount/{amount}`: Send a new transaction to the node `{address}`. The transaction moves an amount of coins from one address (`{from_address}`) to another (`{to_address}`).

## Examples

Start a demo, creating some nodes and periodically sending random transactions:
```bash
cargo run -- --port=1234 --demo
```

Manual demo:
```bash
cargo run -- 1234 &
NODE_1=$(POST http://127.0.0.1:1234/node < /dev/null)
NODE_2=$(POST http://127.0.0.1:1234/node < /dev/null)
GET -s http://127.0.0.1:1234/node/$NODE_1
GET -s http://127.0.0.1:1234/node/$NODE_1/blockchain_balance
GET -s http://127.0.0.1:1234/node/$NODE_1/mempool_balance
POST -s http://127.0.0.1:1234/node/$NODE_1/send/from/$NODE_1/to/$NODE_3/amount/0 < /dev/null
POST -s http://127.0.0.1:1234/node/$NODE_1/send/from/$NODE_1/to/$NODE_3/amount/-1 < /dev/null
POST -s http://127.0.0.1:1234/node/$NODE_1/send/from/$NODE_1/to/$NODE_3/amount/1 < /dev/null
POST -s http://127.0.0.1:1234/node/$NODE_1/send/from/$NODE_1/to/$NODE_3/amount/99999999 < /dev/null
```

## Code Structure

Except for `main`, each file corresponds to a Rust module:
* `src/main.rs`: The entry point of the program. It parses command line arguments and starts the server.
* `src/prelude.rs`: A module that re-exports commonly used items.
* `src/server.rs`: The REST API server.
* `src/constants.rs`: Definition of some constants, such as the difficulty of the proof-of-work.
* `src/block.rs`: The definition of the addresses, blocks and transactions.
* `src/node.rs`: The definition of a node. It includes the logic to mine new blocks, reach consensus and react to other nodes.
* `src/blockchain.rs`: The definition of the local blockchain of a node.
* `src/mempool.rs`: The definition of the pool of pending transactions of a node.
* `src/world.rs`: The definition of the singleton that holds all the nodes of the simulation.
* `src/network.rs`: The definition of the network that the nodes use to communicate with each other.
