use serde_json::Value;
use reqwest;

use core::f64;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync >>;
use std::{env, fs};

fn submit_price(price: f64, sign:String) -> Result<()> {
    let _client = reqwest::Client::new();
    // let resp = client.get("https://api.coinbase.com/v2/prices/spot?currency=USD").send();
    // if let Ok(mut resp) = resp {
    //     let rstring = resp.text().unwrap();
    //     let resp:Value = serde_json::from_str(rstring.as_str())?;
    //     // println!("{:#?}", resp);   
    // }
    println!("submitted price to tezos. Price: {:?},sign:{:?}", price, sign);
    Ok(())
}
fn main() -> Result<()> {

    if env::args().nth(1).is_none() {
        panic!("Too few arguments");
    }

    let price = env::args().nth(1).unwrap();
    let price_float = price.parse::<f64>().unwrap();

    let data = fs::read_to_string("signature")
        .expect("Unable to read signature, make sure it is generated by the signer");

    let (_r, r_value, _s, s_value,_v, v_value): (
        String,
        String,
        String,
        String,
        String,
        u8
    ) = serde_json::from_str(&data).unwrap();
    
    let sign = format!("0x{}{}{}",r_value,s_value,v_value);
    
    let _result = submit_price(price_float, sign).unwrap();
    
    Ok(())
}