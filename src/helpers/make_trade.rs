use alloy::signers::{ local::LocalSigner};
use polymarket_client_sdk::{
    auth::{Normal, state::Authenticated},
    clob::Client,
    clob::types::{ Amount, Side},
};
use rust_decimal::Decimal;
use serde::Deserialize;

use alloy::signers::k256::ecdsa::SigningKey;
#[derive(Debug, Deserialize)]
struct OrderResponse {
    error_msg: Option<String>,
    making_amount: f64,
    taking_amount: f64,
    order_id: String,
    success: bool,
    transaction_hashes: Vec<String>,
    trade_ids: Vec<String>,
}

pub async fn make_trade(
    signer: &LocalSigner<SigningKey>,
    client: &Client<Authenticated<Normal>>,
    token_id: &str,
    amount: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Placeholder for trade-making logic
    let amount_decimal = Decimal::try_from(amount)?;
    let market_order = client
        .market_order()
        .token_id(token_id)
        .amount(Amount::usdc(amount_decimal)?)
        .side(Side::Buy)
        .build()
        .await?;
    let signed_order = client.sign(signer,market_order).await?;
    let posted_order = client.post_order(signed_order).await?;
    for response in &posted_order {
        println!("Order response: {:?}", response);
        match &response.error_msg {
            Some(msg) if msg.is_empty() => {
                println!("Order successful: spent: {}$ recieved {} shares, orderId: {}", response.making_amount, response.taking_amount, response.order_id);
            }
            Some(msg) => {
                println!("Error message: {}", msg);
                panic!("Order failed with error");
            }
            None => {
                println!("Order successful with no error message");
            }
        }

    }   
    Ok(())
}
