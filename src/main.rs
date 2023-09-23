use core::panic;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use sha1::{Digest, Sha1};

mod cli;
mod decode;
mod download_piece;
mod handshake;
mod info;
mod peers;

#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    name: String,
    pieces: ByteBuf,
    #[serde(rename = "piece length")]
    piece_length: u32,
    length: Option<u32>,
}

impl Info {
    pub fn hash_str(&self) -> String {
        let hash = self.hash_bytes();
        to_hex_string(&hash.to_vec())
    }

    pub fn hash_bytes(&self) -> [u8; 20] {
        let info_encoded_value = serde_bencode::to_bytes(&self).unwrap();
        calculate_hash(&info_encoded_value)
            .try_into()
            .expect("Could not convert Vec<u8> to [u8; 20]")
    }

    fn get_pieces_count(&self) -> usize {
        self.pieces.chunks(20).count()
    }

    fn get_piece_hashes(&self) -> Vec<Vec<u8>> {
        self.pieces.chunks(20).map(|chunk| chunk.to_vec()).collect()
    }

    pub fn get_piece_hashes_str(&self) -> Vec<String> {
        self.get_piece_hashes()
            .iter()
            .map(|piece_hash| to_hex_string(&piece_hash))
            .collect()
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
    async fn from(torrent_metadata: &TorrentMetadata) -> Self {
        let info_hash = torrent_metadata.info.hash_bytes();

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
        let res = client.get(url).query(query).send().await.unwrap();

        // println!("status_code: {}", res.status());

        let res_bytes = res.bytes().await.unwrap();
        // println!("res_bytes: {:?}", res_bytes);

        serde_bencode::from_bytes(&res_bytes).expect("TranckerResponse could not be decoded.")
    }
}

impl TrackerResponse {
    #[allow(dead_code)]
    fn get_peers(&self) -> Vec<String> {
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

struct Connection {
    stream: TcpStream,
}

#[allow(dead_code)]
impl Connection {
    fn new(peer: String) -> Self {
        let stream = TcpStream::connect(peer).expect("tcp connection failed!");

        Self { stream }
    }

    fn handshake(&mut self, info_hash: Vec<u8>, peer_id: String) -> Vec<u8> {
        assert_eq!(peer_id.len(), 20, "peer_id should be 20 characters long");

        // Length of the protocol string (1 Byte)
        let mut message: Vec<u8> = vec![19];
        // protocol string (19 Bytes)
        message.extend(b"BitTorrent protocol");
        // eight reserved bytes, all zeros (8 Bytes)
        message.extend(&[0; 8]);
        // sha1 info_hash (20 Bytes)
        message.extend(info_hash);
        // peer id (20 Bytes)
        message.extend(b"00112233445566778899");

        let message_length = self.stream.write(&message).unwrap();

        let mut res_message: Vec<u8> = vec![0; message_length];
        let res_message_length = self.stream.read(&mut res_message).unwrap();

        let res_peer_id = &res_message[res_message_length - 20..];

        res_peer_id.to_vec()
    }

    fn wait(&mut self, message_type: PeerMessageType) -> Vec<u8> {
        let wait_messages_type = vec![
            PeerMessageType::BitField,
            PeerMessageType::Unchoke,
            PeerMessageType::Piece,
        ];

        if wait_messages_type.contains(&message_type) {
            let mut recv_message_length_buf: [u8; 4] = [0; 4];
            self.stream
                .read_exact(&mut recv_message_length_buf)
                .expect("Could not read length buffer");

            let recv_message_length = u32::from_be_bytes(recv_message_length_buf);
            println!("recv_message_length: {}", recv_message_length);

            let mut recv_message_id_buf: [u8; 1] = [0; 1];
            self.stream
                .read_exact(&mut recv_message_id_buf)
                .expect("Could not read message id");

            let recv_message_id = u8::from_be_bytes(recv_message_id_buf);
            let recv_message_type = PeerMessageType::from(recv_message_id);
            println!("recv_message_type: {:?}", recv_message_type);

            assert_eq!(
                message_type, recv_message_type,
                "Message type expected {:?} but received {:?}",
                message_type, recv_message_type
            );

            let payload_length = recv_message_length as usize - 1;
            let mut payload_buf: Vec<u8> = vec![0; payload_length];
            self.stream
                .read_exact(&mut payload_buf)
                .expect("Could not read payload");

            println!("received payload length: {}", payload_buf.len());

            return payload_buf;
        } else {
            panic!("Can not wait for this message type: {:?}", message_type);
        }
    }

    fn send(&mut self, message_type: PeerMessageType, payload: Vec<u8>) {
        let send_messages_type = vec![PeerMessageType::Interested, PeerMessageType::Request];

        if send_messages_type.contains(&message_type) {
            let message_length = (payload.len() + 1) as u32;
            let message_length_buf = message_length.to_be_bytes();
            let message_id = message_type.to_message_id();
            let message_id_buf = message_id.to_be_bytes();

            let mut message: Vec<u8> = Vec::new();
            message.extend(message_length_buf);
            message.extend(message_id_buf);
            message.extend(payload);

            self.stream
                .write_all(&message)
                .expect("Could not send the message");
            println!("Sent message type: {:?}", message_type);
        } else {
            panic!("Can not send this type of message: {:?}", message_type);
        }
    }

    fn download_block(
        &mut self,
        piece_index: u32,
        block_byte_offset: u32,
        block_length: u32,
    ) -> Block {
        println!(
            ">> Sending request for block {} of piece {}",
            block_byte_offset, piece_index
        );
        // 4. Send a `request` message
        let mut request_payload: Vec<u8> = Vec::new();
        request_payload.extend(piece_index.to_be_bytes());
        request_payload.extend(block_byte_offset.to_be_bytes());
        request_payload.extend(block_length.to_be_bytes());

        self.send(PeerMessageType::Request, request_payload);

        // 5. Wait for `piece` message
        let block_data = self.wait(PeerMessageType::Piece);

        let block = Block::from(block_data);

        let recv_block_data_length = block.block_data.len() as u32;

        assert_eq!(
            recv_block_data_length, block_length,
            "Block length expected {} and received {} are not same.",
            block_length, recv_block_data_length
        );

        println!(
            ">> Downloaded block of byte offset {} of block data length: {}",
            block_byte_offset, recv_block_data_length
        );

        block
    }

    fn download_piece(&mut self, piece_index: u32, piece_length: u32) -> Piece {
        println!("> Downloading piece {}", piece_index);

        let block_length: u32 = 16 * 1024;
        // let blocks_count = (piece_length / block_length) + 5 - block_length * (piece_length / block_length);
        // println!(">> Piece {} contains {} blocks", piece_index, blocks_count);

        let mut blocks: Vec<Block> = Vec::new();

        let mut block_index = 0 as u32;
        let mut block_byte_offset;

        // let mut piece_data: Vec<u8> = Vec::new();

        loop {
            block_byte_offset = block_index * block_length;
            if block_byte_offset >= piece_length - 1 {
                break;
            }
            println!("block_byte_offset: {}", block_byte_offset);

            let is_last_block = piece_length - 1 - block_byte_offset < block_length;

            let actual_block_length = if is_last_block {
                piece_length - block_byte_offset
            } else {
                block_length
            };

            println!("actual_block_length: {}", actual_block_length);

            let block = self.download_block(piece_index, block_byte_offset, actual_block_length);

            // piece_data.extend(&block.block_data);
            blocks.push(block);

            block_index += 1;
        }

        // let piece_hash = calculate_hash(&piece_data);
        // println!("* calculated piece_hash: {:?}", piece_hash);

        Piece::from(blocks)
    }

    fn download_blocks_pipelined(
        &mut self,
        no_of_requests: usize,
        piece_index: u32,
        piece_length: u32,
        block_index: &mut u32,
    ) -> Vec<Block> {
        let mut no_of_requests_sent = 0 as usize;
        let mut blocks: Vec<Block> = Vec::new();

        let mut block_byte_offset;
        let block_length = 16 * 1024 as u32;
        // let mut actual_block_lengths: Vec<u32> = vec![];
        let mut actual_block_lengths = HashMap::new();

        for _ in 0..no_of_requests {
            block_byte_offset = *block_index * block_length;
            if block_byte_offset >= piece_length - 1 {
                break;
            }

            println!(
                ">> Sending request for block byte offset {} of piece {}",
                block_byte_offset, piece_index
            );

            let is_last_block = piece_length - 1 - block_byte_offset < block_length;

            println!("is_last_block: {}", is_last_block);
            println!("piece data left: {}", piece_length - block_byte_offset);

            let actual_block_length = if is_last_block {
                piece_length - block_byte_offset
            } else {
                block_length
            };
            println!("actual_block_length: {}", actual_block_length);

            // actual_block_lengths.push(actual_block_length);
            actual_block_lengths.insert(block_byte_offset, actual_block_length);

            // 4. Send a `request` message
            let mut request_payload: Vec<u8> = Vec::new();
            request_payload.extend(piece_index.to_be_bytes());
            request_payload.extend(block_byte_offset.to_be_bytes());
            request_payload.extend(actual_block_length.to_be_bytes());

            self.send(PeerMessageType::Request, request_payload);

            *block_index += 1;
            no_of_requests_sent += 1;
        }

        for _ in 0..no_of_requests_sent {
            println!(">> Waiting for next block");
            let block_data = self.wait(PeerMessageType::Piece);

            let block = Block::from(block_data);
            let recv_block_length = block.block_data.len() as u32;

            let actual_block_length = *actual_block_lengths.get(&block.block_byte_offset).expect(
                format!(
                    "Could not get value for the key: {}",
                    block.block_byte_offset
                )
                .as_str(),
            );
            assert_eq!(
                recv_block_length, actual_block_length,
                "Block length expected {} and received {} are not same.",
                actual_block_length, recv_block_length
            );

            println!(
                ">> block of byte offset {} downloaded of block length: {}",
                block.block_byte_offset, recv_block_length
            );

            blocks.push(block);
        }

        blocks
    }

    fn download_piece_pipelined(
        &mut self,
        no_of_requests: usize,
        piece_index: u32,
        piece_length: u32,
    ) -> Piece {
        println!("> Downloading piece {}", piece_index);

        let block_length = 16 * 1024 as u32;
        // let blocks_count = (piece_length / block_length) + 5 - block_length * (piece_length / block_length);
        // println!(">> Piece {} contains {} blocks", piece_index, blocks_count);

        let mut blocks: Vec<Block> = Vec::new();

        let mut block_index = 0 as u32;

        loop {
            let block_byte_offset = block_index * block_length;
            if block_byte_offset >= piece_length - 1 {
                break;
            }

            let next_blocks = self.download_blocks_pipelined(
                no_of_requests,
                piece_index,
                piece_length,
                &mut block_index,
            );

            blocks.extend(next_blocks);
        }

        Piece::from(blocks)
    }
}

#[derive(Debug, PartialEq)]
enum PeerMessageType {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    BitField,
    Request,
    Piece,
    Cancel,
}

impl From<u8> for PeerMessageType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Choke,
            1 => Self::Unchoke,
            2 => Self::Interested,
            3 => Self::NotInterested,
            4 => Self::Have,
            5 => Self::BitField,
            6 => Self::Request,
            7 => Self::Piece,
            8 => Self::Cancel,
            _ => panic!("Invalid message id"),
        }
    }
}

impl PeerMessageType {
    fn to_message_id(&self) -> u8 {
        match self {
            Self::Choke => 0,
            Self::Unchoke => 1,
            Self::Interested => 2,
            Self::NotInterested => 3,
            Self::Have => 4,
            Self::BitField => 5,
            Self::Request => 6,
            Self::Piece => 7,
            Self::Cancel => 8,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Block {
    piece_index: u32,
    block_byte_offset: u32,
    block_data: Vec<u8>,
}

impl Block {
    fn from(payload: Vec<u8>) -> Self {
        // extract piece index buffer from payload
        let piece_index_buf = payload
            .get(..4)
            .expect("Not able to extract 4 bytes of piece index from payload");

        // decode the buffer into piece index
        let piece_index = u32::from_be_bytes(
            piece_index_buf
                .try_into()
                .expect("Not able to convert the extracted 4 bytes to piece index"),
        );

        // extract block byte offset buffer from payload
        let block_byte_offset_buf = payload
            .get(4..8)
            .expect("Not able to extract the next 4 bytes for block byte offset from payload");
        // decode the buffer into block byte offset
        let block_byte_offset = u32::from_be_bytes(
            block_byte_offset_buf
                .try_into()
                .expect("Not able to convert the extracted 4 bytes to block byte offset"),
        );

        let block_data = payload
            .get(8..)
            .expect("Not able to extract block data from the payload")
            .to_vec();

        let block_data_length = block_data.len() as u32;

        let block_response_payload = Self {
            piece_index,
            block_byte_offset,
            block_data,
        };

        let required_block_data_length = (payload.len() - 8) as u32;

        assert_eq!(
            block_data_length, required_block_data_length,
            "Invalid block data length, expected {} recieved {}",
            required_block_data_length, block_data_length
        );

        block_response_payload
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Piece {
    piece_index: u32,
    piece_data: Vec<u8>,
}

impl Piece {
    fn from(blocks: Vec<Block>) -> Self {
        let mut piece_data: Vec<u8> = Vec::new();

        let mut blocks = blocks.clone();

        blocks.sort_by(|b1, b2| {
            if b1.block_byte_offset < b2.block_byte_offset {
                Ordering::Less
            } else if b1.block_byte_offset == b2.block_byte_offset {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        });
        blocks.iter().for_each(|block| {
            piece_data.extend(block.block_data.clone());
        });

        let piece_index = blocks
            .get(0)
            .expect("Could not get block from blocks array")
            .piece_index;

        Self {
            piece_index,
            piece_data,
        }
    }

    fn get_hash(&self) -> Vec<u8> {
        calculate_hash(&self.piece_data)
    }
}

fn to_hex_string(bytes: &Vec<u8>) -> String {
    let mut s = String::new();
    for byte in bytes {
        s += format!("{:02x}", byte).as_str();
    }
    s
}

fn calculate_hash(bytes: &Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let hash = hasher.finalize();

    hash.to_vec()
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

#[tokio::main]
async fn main() {
    cli::parse_and_execute().await;
}
