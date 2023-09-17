use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

pub mod cli;
pub mod decode;
pub mod download_piece;
pub mod handshake;
pub mod info;
pub mod peers;

#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    name: String,
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piece_length: i64,
    length: Option<i64>,
}

impl Info {
    pub fn hash_str(&self) -> String {
        let hash = self.hash_bytes();
        to_hex_string(&hash)
    }

    pub fn hash_bytes(&self) -> [u8; 20] {
        let info_encoded_value = serde_bencode::to_bytes(&self).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(info_encoded_value);
        let hash = hasher.finalize();

        hash.into()
    }

    pub fn print_peice_hashes(&self) {
        self.pieces.chunks(20).for_each(|piece_hash| {
            let hash: Vec<_> = piece_hash
                .iter()
                .map(|byte| format!("{:02x}", byte))
                .collect();
            println!("{}", hash.join(""));
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TorrentMetadata {
    announce: String,
    info: Info,
}

impl TorrentMetadata {
    /// Read torrent file from given path and generates TorrentMetadata
    pub fn from_file(file_path: PathBuf) -> Self {
        let file_contents = std::fs::read(file_path).expect("Not able to read torrent file.");

        // let decoded_value = decode_bencoded_value_serde_bencode(&file_contents);

        // println!("{}", decoded_value.to_string());

        serde_bencode::from_bytes(&file_contents).unwrap()
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
