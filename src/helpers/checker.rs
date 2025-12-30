use std::collections::HashMap;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub asset: String,
    pub bio: String,
    pub condition_id: String,
    pub event_slug: String,
    pub icon: String,
    pub name: String,
    pub outcome: String,
    pub outcome_index: u32,
    pub price: f64,
    pub proxy_wallet: String,
    pub side: String,
    pub size: f64,
    pub timestamp: u64,
    pub title: String,
    pub transaction_hash: String,
}

#[derive(Deserialize)]
struct FullPayload {
    payload: Payload
}

impl Default for Payload {
    fn default() -> Self {
        Payload { 
            asset: String::from(""), 
            bio: String::from(""), 
            condition_id: String::from(""), 
            event_slug: String::from(""), 
            icon: String::from(""), 
            name: String::from(""), 
            outcome: String::from(""), 
            outcome_index: 0, 
            price: 0.0, 
            proxy_wallet: String::from(""), 
            side: String::from(""), 
            size: 0.0, 
            timestamp: 1, 
            title: String::from(""), 
            transaction_hash: String::from("") 
        }
    }
}

pub fn check(msg: Message, against: &HashMap<String, f64>) -> (bool, Option<String>, Option<String>) {
    let text_message: Utf8Bytes = msg.into_text().unwrap();
    let text_string: &str = text_message.as_str();    
    match serde_json::from_str::<FullPayload>(text_string) {
        Ok(msg_json) => {
            let address =  msg_json.payload.proxy_wallet.to_lowercase();
            let username = msg_json.payload.name.to_lowercase();
            if msg_json.payload.side != "Buy" {
                return (false, None, None);
            }
            if against.contains_key(&username) {
                (true,  Some(msg_json.payload.asset), Some(username))
            } else if against.contains_key(&address) {
                (true, Some(msg_json.payload.asset), Some(address))
            }else {
                (false, None, None)
            } 
            
        }
        Err(e) => {
            eprintln!("Failed to parse message: {} {}", e, &text_string);
            (false, Some(Payload::default().asset), None)
        }
    }
}