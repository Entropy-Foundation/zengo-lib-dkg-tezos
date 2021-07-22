use serde_json::{Value, json};
use reqwest::{self, Client};
use regex::Regex;
use crypto::{blake2b::Blake2b, mac::Mac,ed25519};

use core::f64;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync >>;
use std::{array, collections::HashMap, env, fs, vec};

struct TezosSignature {
    sbytes: String,
    prefix_sign: String
}

fn submit_price(price: f64, sign:String) -> Result<()> {
    let client = reqwest::Client::new();
    // let resp = client.get("https://florencenet.smartpy.io/chains/main/blocks").send();
    // if let Ok(mut resp) = resp {
    //     let rstring = resp.text().unwrap();
    //     let resp:Value = serde_json::from_str(rstring.as_str())?;
    //     // println!("{:#?}", resp); 
    //     let block_hash_value = &resp[0][0];
    //     let block_hash = block_hash_value.as_str().unwrap();

    //     let url_for_contract = format!("https://florencenet.smartpy.io/chains/main/blocks/{}/context/contracts/{}/storage",block_hash,contract_add);

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
    let manager = get_manager(&client, public_key);
    let counter = get_counter(&client, public_key);


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
            "source":"",
            "counter":""
        }"#;

    let mut op_obj:Value = serde_json::from_str(data)?;
    op_obj["destination"] = Value::from(contract_add);
    op_obj["source"] = Value::from(public_key);
    let mut counter_val = counter.parse::<u32>().expect("counter not parsed");
    counter_val += 1;
    op_obj["counter"] = Value::from(counter_val.to_string());

    let send_op_str = r#"{
        "chain_id":"abc",
        "operation":"abc"
    }"#;

    let full_op_str = r#"{
        "branch":"",
        "contents":""
    }"#;

    let mut full_op_obj:Value = serde_json::from_str(full_op_str)?;

    full_op_obj["branch"] = Value::from(head_hash);
    full_op_obj["contents"] = json!([op_obj]);

    get_operation_bytes(full_op_obj.clone());

    

    let mut send_op_obj:Value = serde_json::from_str(send_op_str)?;

    send_op_obj["chain_id"] = Value::from(chain_id);
    send_op_obj["operation"] = full_op_obj;


    let result = simulate_operation(&client, send_op_obj.clone());

    if result["contents"][0]["metadata"]["operation_result"]["status"].as_str().unwrap() == "applied" {
        let consumed_gas = result["contents"][0]["metadata"]["operation_result"]["consumed_gas"].as_str().unwrap().parse::<u32>().unwrap();
        let storage_size = result["contents"][0]["metadata"]["operation_result"]["storage_size"].as_str().unwrap().parse::<u32>().unwrap();

        send_op_obj["operation"]["contents"][0]["gas_limit"] = Value::from((consumed_gas+100).to_string());
        send_op_obj["operation"]["contents"][0]["storage_limit"] = Value::from((storage_size+20).to_string());
    }
    
    let forge_operation = forge_operation(&client, send_op_obj["operation"].clone(), head_hash);

    let signops = tezos_sign(forge_operation.as_str().unwrap(), private_key);

    send_op_obj["operation"]["signature"] = Value::from(signops.prefix_sign);

    let pre_apply_operation = pre_apply_operation(&client, send_op_obj["operation"].clone(), next_protocol);

    if pre_apply_operation[0]["contents"][0]["metadata"]["operation_result"]["status"] == "applied" {
        let body = format!("{:?}",signops.sbytes);
        let op_resp = client.post("https://florencenet.smartpy.io/injection/operation").body(body).send();
        if let Ok(mut op_resp) = op_resp {
            let op_hash = op_resp.text().unwrap();
            println!("submitted price to tezos. Price: {:?},sign:{:?}", price, sign);
            println!("operation_hash:{:?}", op_hash);
        }
    } else {
        println!("Unable to submit the transaction");
    }

    Ok(())
}

fn get_head(client:&Client) -> Value
{
    let resp = client.get("https://florencenet.smartpy.io/chains/main/blocks/head/header").send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn get_metadata(client:&Client) -> Value
{
    let resp = client.get("https://florencenet.smartpy.io/chains/main/blocks/head/metadata").send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn get_manager(client:&Client, contract_address: &str) -> String
{
    let url = format!("https://florencenet.smartpy.io/chains/main/blocks/head/context/contracts/{}/manager_key",&contract_address);
    let resp = client.get(&url).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        return resp.clone();
    }
    return "".to_string();
}

fn get_counter(client:&Client, contract_address: &str) -> String
{
    let url = format!("https://florencenet.smartpy.io/chains/main/blocks/head/context/contracts/{}/counter",&contract_address);
    let resp = client.get(&url).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let newstr = resp.replace("\n", "").replace("\"", "");
        return newstr.clone();
    }
    return "".to_string();
}

fn simulate_operation(client:&Client,mut op_obj:Value) -> Value
{
    let signature= "edsigtXomBKi5CTRf5cjATJWSyaRvhfYNHqSUGrn4SdbYRcGwQrUGjzEfQDTuqHhuA8b2d8NarZjz8TRf65WkpQmo423BtomS8Q";

    op_obj["operation"]["signature"] = Value::from(signature);

    let mut body:Value = op_obj.clone();
    let body_str = body.to_string();

    let url = format!("https://florencenet.smartpy.io/chains/main/blocks/head/helpers/scripts/run_operation");
    let resp = client.post(&url).header("Content-Type", "application/json").body(body_str.clone()).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        println!("resp:{:?}", &json_resp);
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn get_operation_bytes(full_op_obj:Value)
{
    
    let branch = full_op_obj["branch"].as_str().unwrap();
    let forge_buffer = bs58_decode(branch, 2);
    let forge_bytes = buf2hex(forge_buffer.clone());
}

fn bs58_decode(string: &str ,length: u32) -> Vec<u8>
{
    let abc = bs58::decode(string).with_check(None).into_vec().unwrap();
    println!("bs58decode: {:?}",&abc);
    println!("bs58decode: {:?}",&abc[length as usize..].to_vec());
    return abc[length as usize..].to_vec();
}

fn forge_operation(client:&Client, op_obj:Value, head_hash: &str) -> Value
{
    let mut body:Value = op_obj.clone();
    let body_str = body.to_string();
    println!("forge body string:{:?}", &body_str);
    let url = format!("https://florencenet.smartpy.io/chains/main/blocks/{}/helpers/forge/operations", head_hash);
    let resp = client.post(&url).header("Content-Type", "application/json").body(body_str.clone()).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        println!("forge resp:{:?}", &json_resp);
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn pre_apply_operation(client:&Client, mut op_obj:Value, protocol:&str) -> Value
{
    op_obj["protocol"] = Value::from(protocol);
    let mut body:Value = json!([op_obj.clone()]);
    let body_str = body.to_string();
    println!("{:?}", &body_str);
    let url = format!("https://florencenet.smartpy.io/chains/main/blocks/head/helpers/preapply/operations");
    let resp = client.post(&url).header("Content-Type", "application/json").body(body_str.clone()).send();
    if let Ok(mut resp) = resp {
        let resp = resp.text().unwrap();
        let json_resp:Value = serde_json::from_str(resp.as_str()).unwrap();
        println!("pre apply resp:{:?}", &json_resp);
        return json_resp;
    }
    return serde_json::from_str("[]").unwrap();
}

fn tezos_sign(bytes: &str,sk:&str) -> TezosSignature
{
    let curve = &bytes[0..2];

    let encrypted = &bytes[2..3] == "e";

    let constructed_key = bs58_decode(sk.clone(), 4); 

    let secret_key;
    // if constructed_key.len() == 64 {
        secret_key = constructed_key;
    // } else {
    //     let (secret, public) = ed25519::keypair(&constructed_key);
    //     secret_key = secret.to_vec();
    // }

    // later this can be used to decrypt secret with password
    // if encrypted {

    // }

    

    let byteshex = hex2buf(bytes);
    // bb.push(3);//magic bytes
    let mut bb = [3].to_vec();
    for i in byteshex {
        bb.push(i);
    }

    println!("buffer: {:?}",&bb);
    let mut b2b = Blake2b::new(32);
    b2b.input(&bb);
    let byte_hash = b2b.result().code().to_vec();

    println!("secret key: {:?}",&secret_key);
    
    let signature = ed25519::signature(&byte_hash, &secret_key).to_vec();

    println!("signature: {:?}",&signature);

    let sbytes = format!("{}{}",bytes, buf2hex(signature.clone()));

    let prefix_hash = bs58_encode(signature.clone(), [9, 245, 205, 134, 18].to_vec());
    println!("prefixhash: {:?}", &prefix_hash);
    
    
    println!("byte hash: {:?}",byte_hash);
    println!("signature bytes: {:?}",sbytes);

    TezosSignature {
        sbytes: sbytes.to_string(),
        prefix_sign: prefix_hash.to_string()
    }
    
}

fn bs58_encode(payload:Vec<u8>, prefix_arg:Vec<u8>) -> String
{
    let mut actual_payload = prefix_arg;
    for i in payload {
        actual_payload.push(i);
    }
    let _input = buf2hexarr(actual_payload.clone());
    
    bs58::encode(actual_payload).with_check().into_string()
}

fn hex2buf(hex:&str) -> Vec<u8>
{
    let re = Regex::new(r"([\da-f]{2})").unwrap();
    let buff:Vec<u8> = re.captures_iter(hex).map(|c| u8::from_str_radix(c.get(1).unwrap().as_str(),16).unwrap()).collect();   
    buff
}

fn buf2hex(buf:Vec<u8>) -> String
{
    let mut tmp_bytes = Vec::new();
    for i in buf {
        let tmp = format!("{:02x}",i);
        tmp_bytes.push(tmp);
    }
    let strng = tmp_bytes.join("");

    return strng
}

fn buf2hexarr(buf:Vec<u8>) 
{
    let mut tmp_bytes = Vec::new();
    for i in buf {
        let tmp = format!("{:02x}",i);
        tmp_bytes.push(tmp);
    }
    println!("buf2hexarr: {:?}", &tmp_bytes);
    // tmp_bytes.to_vec().clone();
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
        u32
    ) = serde_json::from_str(&data).unwrap();
    
    let sign = format!("0x{}{}{:02x}",r_value,s_value,v_value);
    
    let _result = submit_price(price_float, sign).unwrap();
    
    Ok(())
}