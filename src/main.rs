use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode::de;

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    // If encoded_value starts with a digit, it's a number
    if encoded_value.chars().next().unwrap().is_digit(10) {
        // Example: "5:hello" -> "5"
        let colon_index = encoded_value.find(':').unwrap();
        // let number_string = &encoded_value[..colon_index];
        // let string_length = number_string.parse::<i64>().unwrap();
        // let string = &encoded_value[colon_index + 1..colon_index + 1 + string_length as usize];
        let num_string = &encoded_value[colon_index + 1..];
        return serde_json::Value::String(num_string.to_string());
    } else if encoded_value.starts_with("i") && encoded_value.ends_with("e") {
        // let e_index = encoded_value.find('e').unwrap();
        // let number_string = &encoded_value[1..e_index];
        // let number = number_string.parse::<i64>().unwrap();
        let number = encoded_value
            .strip_prefix("i").unwrap()
            .strip_suffix("e").unwrap()
            .parse::<i64>().unwrap();
        return serde_json::Value::Number(number.into());
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

#[allow(dead_code)]
fn decode_bencoded_value_serde_bencode(encoded_value: &str) -> serde_json::Value {
    let value: serde_bencode::value::Value = serde_bencode::from_str(encoded_value).unwrap();
    match value {
        serde_bencode::value::Value::Bytes(b) => {
            serde_json::Value::String(String::from_utf8(b).unwrap())
        },
        serde_bencode::value::Value::Int(i) => serde_json::Value::Number(i.into()),
        serde_bencode::value::Value::List(_) => todo!(),
        serde_bencode::value::Value::Dict(_) => todo!(),
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        // let decoded_value = decode_bencoded_value(encoded_value);
        let decoded_value = decode_bencoded_value_serde_bencode(encoded_value);
        
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
