use clap::Args;
use serde_bencode;
use serde_json;

fn transform_bencode_to_json(value: &serde_bencode::value::Value) -> serde_json::Value {
    match value {
        serde_bencode::value::Value::Bytes(b) => {
            if let Ok(s) = String::from_utf8(b.clone()) {
                serde_json::Value::String(s)
            } else {
                // serde_bytes::ByteBuf::from(b.clone())
                serde_json::Value::Null
            }
            // serde_json::Value::String(String::from_utf8(b.clone()).unwrap())
        }
        serde_bencode::value::Value::Int(i) => serde_json::Value::Number((*i).into()),
        serde_bencode::value::Value::List(l) => {
            let values = l.iter().map(transform_bencode_to_json).collect();
            serde_json::Value::Array(values)
        }
        serde_bencode::value::Value::Dict(d) => {
            let map = d
                .iter()
                .filter_map(|(key, value)| {
                    String::from_utf8(key.clone())
                        .ok()
                        .map(|key_str| (key_str, transform_bencode_to_json(value)))
                })
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

fn decode_bencoded_value_serde_bencode(encoded_value: &[u8]) -> serde_json::Value {
    let value: serde_bencode::value::Value = serde_bencode::from_bytes(encoded_value).unwrap();
    return transform_bencode_to_json(&value);
}

#[derive(Args, Debug)]
pub struct DecodeArgs {
    /// encoded value (String)
    encoded_value: String,
}

pub fn execute(args: &DecodeArgs) {
    // uses serde_bencode for parsing
    let decoded_value = decode_bencoded_value_serde_bencode(args.encoded_value.as_bytes());

    println!("{}", decoded_value.to_string());
}
