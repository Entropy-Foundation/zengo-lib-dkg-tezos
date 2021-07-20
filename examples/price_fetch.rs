use tokio::{task};
use futures::future::join_all;
use serde_json::Value;
use reqwest;

use core::f64;
use std::{sync::{Arc, Mutex}};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync >>;

async fn get_price(prices_vec:Arc<Mutex<Vec<f64>>>) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client.get("https://api.coinbase.com/v2/prices/spot?currency=USD").send();
    if let Ok(mut resp) = resp {
        let rstring = resp.text().unwrap();
        let resp:Value = serde_json::from_str(rstring.as_str())?;

        if let Some(data) = resp.get("data") { 
            if let Some(amount_str) = data["amount"].as_str() {
                let amount = amount_str.parse::<f64>()?;
                prices_vec.lock().unwrap().push(amount);
            }
        }
    }
    // println!("{:#?}", resp);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let prices_vec:Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for _i in 0..13 {
        let price_clone = Arc::clone(&prices_vec);

        let handle = task::spawn(get_price(price_clone));
        handles.push(handle);
    }
    join_all(handles).await;

    let prices_clone = Arc::clone(&prices_vec);

    let medium_price = get_medium_value(prices_clone)?;

    println!("{:?}", 3500);
    // println!("amit");

    Ok(())

}

fn get_medium_value(prices_vec:Arc<Mutex<Vec<f64>>>) -> Result<f64>
{
    prices_vec.lock().unwrap().sort_by(|a,b| a.partial_cmp(b).unwrap());

    let mid = prices_vec.lock().unwrap().len() / 2;

    if let Some(mid_value) = prices_vec.lock().unwrap().get(mid) {
        let mid_value = *mid_value;
        return Ok(mid_value);
    }
    Ok(0.0)
}