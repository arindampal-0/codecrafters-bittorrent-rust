use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_bencode;
use serde_bytes::ByteBuf;
use serde_json;
use sha1::{Digest, Sha1};
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Serialize, Deserialize, Debug)]
struct Info {
    name: String,
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piece_length: i64,
    length: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TorrentMetadata {
    announce: String,
    info: Info,
}

impl TorrentMetadata {
    pub fn from_file(file_path: String) -> Self {
        let file_contents = std::fs::read(file_path).expect("Not able to read torrent file.");

        // let decoded_value = decode_bencoded_value_serde_bencode(&file_contents);

        // println!("{}", decoded_value.to_string());

        serde_bencode::from_bytes(&file_contents).unwrap()
    }

    pub fn hash_str(&self) -> String {
        let hash = self.hash_bytes();
        to_hex_string(&hash)
    }

    pub fn hash_bytes(&self) -> [u8; 20] {
        let info_encoded_value = serde_bencode::to_bytes(&self.info).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(info_encoded_value);
        let hash = hasher.finalize();

        hash.into()
    }

    pub fn print_peice_hashes(&self) {
        self.info.pieces.chunks(20).for_each(|piece_hash| {
            let hash: Vec<_> = piece_hash
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect();
            println!("{}", hash.join(""));
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TrackerResponse {
    interval: u64,
    #[serde(with = "serde_bytes")]
    peers: ByteBuf,
}

impl TrackerResponse {
    #[allow(dead_code)]
    fn print_peers(&self) -> Vec<String> {
        self.peers
            .chunks(6)
            .map(|chunk| {
                let ip = chunk[0..4]
                    .iter()
                    .map(|b| format!("{}", b))
                    .collect::<Vec<String>>()
                    .join(".");

                let port = u16::from_be_bytes([chunk[4], chunk[5]]);

                format!("{}:{}", ip, port)
            })
            .collect()
    }
}

fn to_hex_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    for byte in bytes {
        s += format!("{:02x}", byte).as_str();
    }
    s
}

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

#[allow(dead_code)]
fn decode_bencoded_value_serde_bencode(encoded_value: &[u8]) -> serde_json::Value {
    let value: serde_bencode::value::Value = serde_bencode::from_bytes(encoded_value).unwrap();
    return transform_bencode_to_json(&value);
}

pub fn urlencode_hash(i: &[u8; 20]) -> String {
    i.into_iter()
        .map(|&b| match b {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'.' | b'_' | b'~' => {
                format!("{}", b as char)
            }
            _ => format!("%{:02X}", b),
        })
        .collect::<String>()
}

/// For command: "decode"
fn decode(encoded_value: &String) {
    // uses self-made bencode parser
    // let decoded_value = decode_bencoded_value(encoded_value);

    // uses serde_bencode for parsing
    let decoded_value = decode_bencoded_value_serde_bencode(encoded_value.as_bytes());

    println!("{}", decoded_value.to_string());
}

/// For command: "info"
fn read_torrent_metadata(torrent_file_path: &String) {
    let torrent_metadata = TorrentMetadata::from_file(torrent_file_path.clone());

    println!("Tracker URL: {}", torrent_metadata.announce);
    // println!("Info: {:?}", torrent_metadata.info);
    println!("Length: {}", torrent_metadata.info.length.unwrap());

    println!("Info Hash: {}", torrent_metadata.hash_str());

    println!("Piece Length: {}", torrent_metadata.info.piece_length);

    println!("Piece Hashes:");
    torrent_metadata.print_peice_hashes();
}

// For command: "peers"
async fn get_torrent_peers(torrent_file_path: &String) -> Result<()> {
    let torrent_metadata = TorrentMetadata::from_file(torrent_file_path.clone());

    let info_hash = torrent_metadata.hash_bytes();

    let url = format!(
        "{}?info_hash={}",
        torrent_metadata.announce,
        urlencode_hash(&info_hash)
    );

    let query = &[
        ("peer_id", "00112233445566778899".to_string()),
        ("port", "6881".to_string()),
        ("uploaded", "0".to_string()),
        ("downloaded", "0".to_string()),
        ("left", torrent_metadata.info.length.unwrap().to_string()),
        ("compact", "1".to_string()),
    ];

    let client = reqwest::Client::new();
    let res = client.get(url).query(query).send().await?;

    // println!("status_code: {}", res.status());

    let res_bytes = res.bytes().await?;
    // println!("res_bytes: {:?}", res_bytes);
    let tracker_response: TrackerResponse =
        serde_bencode::from_bytes(&res_bytes).expect("TranckerResponse could not be decoded.");
    // println!("tracker_response: {:?}", tracker_response);

    tracker_response
        .print_peers()
        .iter()
        .for_each(|peer_str| println!("{}", peer_str));

    Ok(())
}

// For command: "handshake"
fn peer_handshake(torrent_file_path: &String, peer_address: &String) {
    let torrent_metadata = TorrentMetadata::from_file(torrent_file_path.clone());
    let mut stream = TcpStream::connect(peer_address).expect("tcp connection failed!");

    // Length of the protocol string (1 Byte)
    let mut message: Vec<u8> = vec![19];
    // protocol string (19 Bytes)
    message.extend(b"BitTorrent protocol");
    // eight reserved bytes, all zeros (8 Bytes)
    message.extend(&[0; 8]);
    // sha1 info_hash (20 Bytes)
    message.extend(torrent_metadata.hash_bytes());
    // peer id (20 Bytes)
    message.extend(b"00112233445566778899");

    let message_length = stream.write(&message).unwrap();

    let mut res_message: Vec<u8> = vec![0; message_length];
    let res_message_length = stream.read(&mut res_message).unwrap();

    let res_peer_id = &res_message[res_message_length - 20..];

    println!("Peer ID: {}", to_hex_string(res_peer_id));
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    // let command = args.get(1).expect("No command specified");
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        decode(encoded_value);
    } else if command == "info" {
        let torrent_file_path = &args[2];
        read_torrent_metadata(torrent_file_path);
    } else if command == "peers" {
        let torrent_file_path = &args[2];
        get_torrent_peers(torrent_file_path).await?;
    } else if command == "handshake" {
        let torrent_file_path = &args[2];
        let peer_address = &args[3];
        peer_handshake(torrent_file_path, peer_address);
    } else {
        println!("unknown command: {}", args[1]);
    }

    Ok(())
}
