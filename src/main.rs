#![deny(unused_must_use)]

use clap::Parser;
use prelude::*;
use rand::prelude::SliceRandom;
use rand::Rng;

mod block;
mod blockchain;
mod constants;
mod mempool;
mod network;
mod node;
mod prelude;
mod server;
mod world;

/// Simulator of a simple blockchain.
#[derive(Parser)]
struct Args {
    /// The port on which the server will listen.
    #[clap(long, short)]
    port: Option<u16>,
    /// Enable the demo mode.
    #[clap(long, short, action)]
    demo: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if args.demo {
        tokio::spawn(async {
            let nodes = [
                world::world().await.add_node().await,
                world::world().await.add_node().await,
                world::world().await.add_node().await,
            ];

            let mut max_amount = 100;
            loop {
                let node_addr = *nodes.choose(&mut rand::thread_rng()).unwrap();
                let source_addr = *nodes.choose(&mut rand::thread_rng()).unwrap();
                let destination_addr = *nodes.choose(&mut rand::thread_rng()).unwrap();
                let amount = rand::thread_rng().gen_range(0..=max_amount);

                let transaction = Transaction::new(source_addr, destination_addr, amount);

                let succeeded = world::world()
                    .await
                    .get_node(node_addr)
                    .await
                    .unwrap()
                    .write()
                    .await
                    .add_client_transaction(transaction)
                    .await
                    .is_ok();

                if succeeded {
                    max_amount += 100;
                } else {
                    max_amount -= 10;
                    max_amount = max_amount.max(100);
                }

                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        });
    }

    server::serve(args.port).await;
}
