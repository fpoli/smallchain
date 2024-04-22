use crate::prelude::*;
use std::collections::HashMap;
use warp::http::StatusCode;
use warp::reject::Rejection;
use warp::reply::json;
use warp::reply::Reply;
use warp::Filter;

pub async fn serve(port: Option<u16>) {
    let root = warp::path!().map(|| "Ok".to_string());
    let list_nodes = warp::path!("nodes").and_then(handle_list_nodes);
    let add_node = warp::path!("node").and_then(handle_add_node);
    let show_node = warp::path!("node" / String).and_then(handle_show_node);
    let show_node_block = warp::path!("node" / String / "block" / String)
        .and_then(handle_show_node_block);
    let show_node_blockchain_balance = warp::path!("node" / String / "blockchain_balance")
        .and_then(handle_show_node_blockchain_balance);
    let show_node_mempool_balance =
        warp::path!("node" / String / "mempool_balance").and_then(handle_show_node_mempool_balance);
    let delete_node = warp::path!("node" / String).and_then(handle_delete_node);
    let send_transaction =
        warp::path!("node" / String / "send" / "from" / String / "to" / String / "amount" / String)
            .and_then(handle_send_transaction);

    let get_routes = warp::get().and(
        root.or(list_nodes)
            .or(show_node)
            .or(show_node_block)
            .or(show_node_blockchain_balance)
            .or(show_node_mempool_balance),
    );
    let post_routes = warp::post().and(add_node.or(send_transaction));
    let del_routes = warp::post().and(delete_node);
    let routes = get_routes
        .or(post_routes)
        .or(del_routes)
        .recover(handle_rejection);

    warp::serve(routes)
        .run(([127, 0, 0, 1], port.unwrap_or(0)))
        .await;
}

#[derive(Debug)]
struct InvalidParameter;

impl warp::reject::Reject for InvalidParameter {}

#[derive(Debug)]
struct InvalidTransaction;

impl warp::reject::Reject for InvalidTransaction {}

/// List the nodes in the world.
async fn handle_list_nodes() -> Result<impl Reply, Rejection> {
    let addresses: Vec<Address> = world().await.get_node_addresses().await;
    Ok(json(&addresses))
}

/// Add a node to the world.
async fn handle_add_node() -> Result<impl Reply, Rejection> {
    let address = world().await.add_node().await;
    Ok(json(&address))
}

/// Show the details of a node.
async fn handle_show_node(raw_address: String) -> Result<impl Reply, Rejection> {
    let address = Address::from_str(&raw_address).map_err(|err| {
        warn!("Failed to parse address {raw_address:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let Some(node) = world().await.get_node(address).await else {
        warn!("Cannot find node {address}");
        return Err(warp::reject::custom(InvalidParameter));
    };
    let readable_node = node.read().await;
    let details: HashMap<String, String> = HashMap::from_iter(vec![
        ("blockchain_length".to_string(), readable_node.blockchain().len().to_string()),
        ("last_block_hash".to_string(), readable_node.blockchain().last_hash().to_string()),
        ("mempool_length".to_string(), readable_node.mempool().len().to_string()),
    ]);
    Ok(json(&details))
}

/// Show a block in the local blockchain of a node.
async fn handle_show_node_block(raw_address: String, raw_hash: String) -> Result<impl Reply, Rejection> {
    let address = Address::from_str(&raw_address).map_err(|err| {
        warn!("Failed to parse address {raw_address:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let Some(node) = world().await.get_node(address).await else {
        warn!("Cannot find node {address}");
        return Err(warp::reject::custom(InvalidParameter));
    };
    let hash = BlockHash::from_str(&raw_hash).map_err(|err| {
        warn!("Failed to parse block hash {raw_hash:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let Some(block) = node.read().await.get_block(&hash) else {
        warn!("Cannot find block {hash} in node {address}");
        return Err(warp::reject::custom(InvalidParameter));
    };
    Ok(json(&block))
}

/// Show the blockchain balance of a node.
async fn handle_show_node_blockchain_balance(raw_address: String) -> Result<impl Reply, Rejection> {
    let address = Address::from_str(&raw_address).map_err(|err| {
        warn!("Failed to parse address {raw_address:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let Some(node) = world().await.get_node(address).await else {
        warn!("Cannot find node {address}");
        return Err(warp::reject::custom(InvalidParameter));
    };
    let readable_node = node.read().await;
    let balance = readable_node.blockchain().balance();
    Ok(json(&balance))
}

/// Show the mempool balance of a node.
async fn handle_show_node_mempool_balance(raw_address: String) -> Result<impl Reply, Rejection> {
    let address = Address::from_str(&raw_address).map_err(|err| {
        warn!("Failed to parse address {raw_address:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let Some(node) = world().await.get_node(address).await else {
        warn!("Cannot find node {address}");
        return Err(warp::reject::custom(InvalidParameter));
    };
    let readable_node = node.read().await;
    let balance = readable_node.mempool().balance();
    Ok(json(&balance))
}

/// Delete a node from the world.
async fn handle_delete_node(address: String) -> Result<impl Reply, Rejection> {
    let address = Address::from_str(&address).map_err(|err| {
        warn!("Failed to parse address {address}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    world().await.delete_node(address).await;
    Ok(StatusCode::OK)
}

/// Send a transaction to a node.
async fn handle_send_transaction(
    raw_node_address: String,
    raw_sender: String,
    raw_recipient: String,
    raw_amount: String,
) -> Result<impl Reply, Rejection> {
    let node_address = Address::from_str(&raw_node_address).map_err(|err| {
        warn!("Failed to parse node address {raw_node_address:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let Some(node) = world().await.get_node(node_address).await else {
        warn!("Cannot find node {node_address}");
        return Err(warp::reject::custom(InvalidParameter));
    };
    let sender = Address::from_str(&raw_sender).map_err(|err| {
        warn!("Failed to parse sender address {raw_sender:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let recipient = Address::from_str(&raw_recipient).map_err(|err| {
        warn!("Failed to parse recipient address {raw_recipient:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let amount = raw_amount.parse::<u64>().map_err(|err| {
        warn!("Failed to parse amount {raw_amount:?}: {err:?}");
        warp::reject::custom(InvalidParameter)
    })?;
    let transaction = Transaction::new(sender, recipient, amount);
    let mut writable_node = node.write().await;
    writable_node
        .add_client_transaction(transaction)
        .await
        .map_err(|_| warp::reject::custom(InvalidTransaction))?;
    Ok(StatusCode::OK)
}

/// Handle errors.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if err.is_not_found() {
        Ok(StatusCode::NOT_FOUND)
    } else if let Some(InvalidParameter) = err.find() {
        Ok(StatusCode::BAD_REQUEST)
    } else if let Some(InvalidTransaction) = err.find() {
        Ok(StatusCode::FORBIDDEN)
    } else {
        error!("Internal server error: {:?}", err);
        Ok(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
