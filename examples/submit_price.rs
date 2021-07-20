use serde_json::{Value, json};
use reqwest::{self, Client};

use core::f64;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync >>;
use std::{array, collections::HashMap, env, fs, vec};

fn submit_price(price: f64, sign:String) -> Result<()> {
    let client = reqwest::Client::new();
    // let resp = client.get("https://testnet-tezos.giganode.io/chains/main/blocks").send();
    // if let Ok(mut resp) = resp {
    //     let rstring = resp.text().unwrap();
    //     let resp:Value = serde_json::from_str(rstring.as_str())?;
    //     // println!("{:#?}", resp); 
    //     let block_hash_value = &resp[0][0];
    //     let block_hash = block_hash_value.as_str().unwrap();

    //     let url_for_contract = format!("https://testnet-tezos.giganode.io/chains/main/blocks/{}/context/contracts/{}/storage",block_hash,contract_add);

    //     println!("url:{}",&url_for_contract);

    //     let res = client.get(&url_for_contract).send();

    //     if let Ok(mut res) = res {
    //         let rstring = res.text().unwrap();
    //         let res:Value = serde_json::from_str(rstring.as_str())?;
    //         println!("{:#?}", res);
    //     }
    // }

    let contract_add = "KT1BcGZeriXJVNod2L2aqtSdLQQH6gCseZRX";
    let private_key = "edskRwEs4xCsWgupRZwDdMAywiGXmbhNfbthtGR8yp6GpsEvHCmjrxF5HuvBDMx8RQoCUbfRykJWfQ3nnmKEBfA4s4Cd1TaAmV";
    let public_key = "tz1dYsNHM2t1DfGJLbWGhiZ17x7EV6J8XcJs";

    // generating operation for tezos
    let head = get_head(&client);
    let head_hash = head["hash"].as_str().unwrap();
    let chain_id = head["chain_id"].as_str().unwrap();
    let metadata = get_metadata(&client);
    let next_protocol = metadata["next_protocol"].as_str().unwrap();
    let manager = get_manager(&client, contract_add);
    let counter = get_counter(&client, contract_add);


    let data = r#"
        {
            "kind": "transaction",
            "fee": "1420",
            "gas_limit": "10600",
            "storage_limit": "300",
            "amount":"0",
            "destination":"",
            "parameters":{ 
                "entrypoint": "decrement", 
                "value": { 
                    "int": "10" 
                } 
            },
            "source":""
        }"#;

    let mut op_obj:Value = serde_json::from_str(data)?;
    op_obj["destination"] = Value::from(contract_add);
    op_obj["source"] = Value::from(public_key);

    let send_op_str = r#"{
        "chain_id":"abc",
        "operation":"abc"
    }"#;

    let full_op_str = r#"{
        "branch":"",
        "contents":"",
        "protocol":""
    }"#;

    let mut full_op_obj:Value = serde_json::from_str(full_op_str)?;

    full_op_obj["branch"] = Value::from(head_hash);
    full_op_obj["contents"] = json!([op_obj]);
    full_op_obj["protocol"] = Value::from(next_protocol);

    get_operation_bytes(full_op_obj.clone());

    

    let mut send_op_obj:Value = serde_json::from_str(send_op_str)?;

    send_op_obj["chain_id"] = Value::from(chain_id);
    send_op_obj["operation"] = full_op_obj;


    let result = simulate_operation(&client, send_op_obj);

    // configure whole json of operation
    // sign operation and create content
    // send content to tezos
    let op_body = "";
    let op_resp = client.post("https://testnet-tezos.giganode.io/injection/operation").body(op_body).send();
    if let Ok(mut op_resp) = op_resp {
        let op_hash = op_resp.text().unwrap();
        println!("submitted price to tezos. Price: {:?},sign:{:?}", price, sign);
        println!("operation_hash:{:?}", op_hash);
    }
    

    // println!("submitted price to tezos. Price: {:?},sign:{:?}", price, sign);
    Ok(())
}

fn get_head(client:&Client) -> Value
{
    let resp = client.get("https://testnet-tezos.giganode.io/chains/main/blocks/head/header").send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn get_metadata(client:&Client) -> Value
{
    let resp = client.get("https://testnet-tezos.giganode.io/chains/main/blocks/head/metadata").send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn get_manager(client:&Client, contract_address: &str) -> Value
{
    let url = format!("https://testnet-tezos.giganode.io/chains/main/blocks/head/context/contracts/{}/manager_key",&contract_address);
    let resp = client.get(&url).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        if resp == "" {
            return serde_json::from_str("[]").unwrap();
        }
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn get_counter(client:&Client, contract_address: &str) -> Value
{
    let url = format!("https://testnet-tezos.giganode.io/chains/main/blocks/head/context/contracts/{}/counter",&contract_address);
    let resp = client.get(&url).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        if resp == "" {
            return serde_json::from_str("[]").unwrap();
        }
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn simulate_operation(client:&Client,mut op_obj:Value) -> Value
{
    let signature= "edsigtXomBKi5CTRf5cjATJWSyaRvhfYNHqSUGrn4SdbYRcGwQrUGjzEfQDTuqHhuA8b2d8NarZjz8TRf65WkpQmo423BtomS8Q";

    op_obj["operation"]["signature"] = Value::from(signature);

    let mut body:Value = op_obj.clone();

    println!("{:?}",body.to_string());
    let body_str = body.to_string();

    let url = format!("https://testnet-tezos.giganode.io/chains/main/blocks/head/helpers/scripts/run_operation");
    let resp = client.post(&url).header("Content-Type", "application/json").body(body_str.clone()).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        // if resp == "" {
        // }
        // let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        println!("resp:{:?}", resp);
        return serde_json::from_str("[]").unwrap();
        // return json_resp;
    } else if let Err(mut resp) = resp {
        // let resp = resp.text().unwrap();
        println!("failed resp:{:?}", resp);
    }
    return serde_json::from_str("[]").unwrap();
}

fn get_operation_bytes(full_op_obj:Value){
    
    // bs58_decode(full_op_obj.as_str().unwrap().clone(), 2);
}

fn bs58_decode(string: &str ,length: u32)
{
    let abc = bs58::decode(string).into_vec().unwrap();
    // println!("{:?}", &abc);
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