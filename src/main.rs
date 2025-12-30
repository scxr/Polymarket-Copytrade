#![allow(dead_code)]

mod helpers;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::str::FromStr;

use alloy::network::EthereumWallet;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::LocalSigner;
use alloy::signers::Signer as _;
use dotenv::dotenv;
use futures::{SinkExt, StreamExt};
use polymarket_client_sdk::clob::{Client, Config};
use polymarket_client_sdk::POLYGON;
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use helpers::approvals::{approval_process, is_fully_approved};
use helpers::checker::check;
use helpers::make_trade::make_trade;

use std::io::{self, Write};

const WS_URL: &str = "wss://ws-live-data.polymarket.com";
const CLOB_URL: &str = "https://clob.polymarket.com";
const DEFAULT_RPC: &str = "https://polygon-rpc.com";

fn load_targets(path: &str) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut targets = HashMap::new();

    for result in rdr.records() {
        let record = result?;
        let username = record.get(0).unwrap_or("").to_string();
        let address = record.get(1).unwrap_or("").to_string();
        let size: f64 = record.get(2).unwrap_or("0").parse()?;

        let (key, label) = if !username.is_empty() {
            (username, "username")
        } else if !address.is_empty() {
            (address, "address")
        } else {
            panic!("Must supply either username or address");
        };

        println!("Adding target {}: {} with size: {}", label, key, size);
        targets.insert(key.to_lowercase(), size);
    }

    Ok(targets)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let private_key = env::var("PRIVATE_KEY")
        .expect("Private key not found, please set PRIVATE_KEY in .env file");
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC.to_string());

    let targets = load_targets("target.csv")?;

    // signers and that
    let signer = LocalSigner::from_str(&private_key)?.with_chain_id(Some(POLYGON));
    let wallet = EthereumWallet::from(signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(rpc_url.parse()?);
    let user_address = signer.address();

    // make sure approved
    if is_fully_approved(&provider, user_address).await? {
        println!("User already approved, skipping...");
    } else {
        println!("User not approved, processing approvals...");
        approval_process(provider.clone(), user_address).await?;
    }

    println!("Address: {}", signer.address());

    // clobbing
    let client = Client::new(CLOB_URL, Config::default())?
        .authentication_builder(&signer)
        .authenticate()
        .await?;

    println!("Ok? : {}", client.ok().await?);

    // wss
    let (ws_stream, _) = connect_async(WS_URL).await?;
    println!("Connected to {}", WS_URL);

    let (mut write, mut read) = ws_stream.split();

    let sub_req = json!({
        "action": "subscribe",
        "subscriptions": [{
            "topic": "activity",
            "type": "trades"
        }]
    });
    write.send(Message::Text(sub_req.to_string().into())).await?;


    // check if we needa buy
    let mut cnt = 0;
    while let Some(msg) = read.next().await {
        let msg = msg?;

        if let Message::Text(_) = &msg {
            cnt += 1;
            let (is_match, token_id, matched_key) = check(msg, &targets);

            if is_match {
                let trade_size = targets[matched_key.as_ref().unwrap()];
                make_trade(&signer, &client, token_id.as_ref().unwrap(), trade_size).await?;
            } else {
                print!("\rNo matching trade found. Total messages processed: {}", cnt); 
                io::stdout().flush().unwrap();

            }
        }
    }

    Ok(())
}   