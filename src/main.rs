use serde_json;
use std::env;

// Available if you need it!
use serde_bencode;

/// Returns the `end_index` of the next datatype
/// 
/// # Arguments
/// * `encoded_value` - string slice for parsing the next datatype
/// * `start_index` - `usize` for inclusive start index
/// * `end_index` - `usize` for inclusive end index
fn get_end_index_for_next_datatype(encoded_value: &str, start_index: usize, end_index: usize) -> usize {
    let mut end_index = end_index;

    let encoded_value_range = &encoded_value[start_index..=end_index];

    // Next String data
    if encoded_value_range.chars().next().unwrap().is_digit(10) {
        let colon_index = encoded_value_range.find(':').unwrap();
        let size = encoded_value_range[..colon_index].parse::<i64>().unwrap() as usize;
        end_index = colon_index + size;
    }
    // Next Int data
    else if encoded_value_range.starts_with("i") {
        end_index = encoded_value_range.find('e').unwrap();
    }
    // Next List data
    else if encoded_value_range.starts_with("l") {
        let mut next_index = 1 as usize;
        loop {
            if encoded_value_range.get(next_index..).unwrap().starts_with("e") {
                break;
            }

            next_index = get_end_index_for_next_datatype(encoded_value, start_index + next_index, end_index) + 1 - start_index;
        }

        end_index = next_index;
    }

    return start_index + end_index;
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    // If encoded_value starts with a digit, it's a string
    if encoded_value.chars().next().unwrap().is_digit(10) {
        // Example: "5:hello" -> "hello"
        let colon_index = encoded_value.find(':').unwrap();
        // let number_string = &encoded_value[..colon_index];
        // let string_length = number_string.parse::<i64>().unwrap();
        // let string = &encoded_value[colon_index + 1..colon_index + 1 + string_length as usize];
        let num_string = &encoded_value[colon_index + 1..];
        return serde_json::Value::String(num_string.to_string());
    // If encoded_value starts with 'i' and ends with 'e', it's a number
    } else if encoded_value.starts_with("i") && encoded_value.ends_with("e") {
        // Example: "i52e" -> 52
        // let e_index = encoded_value.find('e').unwrap();
        // let number_string = &encoded_value[1..e_index];
        // let number = number_string.parse::<i64>().unwrap();
        let number = encoded_value
            .strip_prefix("i")
            .unwrap()
            .strip_suffix("e")
            .unwrap()
            .parse::<i64>()
            .unwrap();
        return serde_json::Value::Number(number.into());
    // If encoded_value starts with 'l' and ends with 'e', it's a list
    } else if encoded_value.starts_with("l") && encoded_value.ends_with("e") {

        let mut values = Vec::new();

        // llle5:helloei2elee
        let mut next_first_index = 1 as usize;
        let end_index = encoded_value.len() - 1;
        while next_first_index < end_index {
            let next_end_index = get_end_index_for_next_datatype(
                encoded_value,
                next_first_index,
                end_index,
            );

            let value = decode_bencoded_value(&encoded_value[next_first_index..=next_end_index]);

            values.push(value);

            next_first_index = next_end_index + 1;
        }

        return serde_json::Value::Array(values);
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

fn transform_bencode_to_json(value: &serde_bencode::value::Value) -> serde_json::Value {
    match value {
        serde_bencode::value::Value::Bytes(b) => {
            serde_json::Value::String(String::from_utf8(b.clone()).unwrap())
        }
        serde_bencode::value::Value::Int(i) => serde_json::Value::Number((*i).into()),
        serde_bencode::value::Value::List(l) => {
            let values = l.iter().map(transform_bencode_to_json).collect();
            serde_json::Value::Array(values)
        }
        serde_bencode::value::Value::Dict(_) => todo!(),
    }
}

#[allow(dead_code)]
fn decode_bencoded_value_serde_bencode(encoded_value: &str) -> serde_json::Value {
    let value: serde_bencode::value::Value = serde_bencode::from_str(encoded_value).unwrap();
    return transform_bencode_to_json(&value);
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
