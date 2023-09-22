use std::io::{Read, Write};
use std::net::TcpStream;
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
    pub pieces: ByteBuf,
    #[serde(rename = "piece length")]
    pub piece_length: u32,
    pub length: Option<u32>,
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

    pub fn get_piece_hashes(&self) -> Vec<Vec<u8>> {
        self.pieces.chunks(20).map(|bytes| bytes.to_vec()).collect()
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
    pub info: Info,
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
pub struct TrackerResponse {
    interval: u64,
    #[serde(with = "serde_bytes")]
    peers: ByteBuf,
}

impl TrackerResponse {
    pub async fn from(torrent_metadata: &TorrentMetadata) -> Self {
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
        let tracker_response: TrackerResponse =
            serde_bencode::from_bytes(&res_bytes).expect("TranckerResponse could not be decoded.");
        // println!("tracker_response: {:?}", tracker_response);

        return tracker_response;
    }

    #[allow(dead_code)]
    pub fn get_peers(&self) -> Vec<String> {
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

pub struct Connection {
    pub stream: TcpStream,
}

impl Connection {
    pub fn new(peer_address: String) -> Self {
        // println!("connetion_address: {}", peer_address);
        let stream = TcpStream::connect(peer_address.clone()).expect("tcp connection failed!");

        Connection { stream }
    }

    pub fn handshake(&mut self, info_hash: Vec<u8>, peer_id: &str) -> Vec<u8> {
        if peer_id.len() != 20 {
            panic!("peer_id should be 20 characters long.");
        }

        // Length of the protocol string (1 Byte)
        let mut message: Vec<u8> = vec![19];
        // protocol string (19 Bytes)
        message.extend(b"BitTorrent protocol");
        // eight reserved bytes, all zeros (8 Bytes)
        message.extend(&[0; 8]);
        // sha1 info_hash (20 Bytes)
        message.extend(info_hash.clone());
        // peer id (20 Bytes)
        message.extend(peer_id.as_bytes());

        let _message_length = self
            .stream
            .write(&message)
            .expect("Handshake: could not send message length");

        // println!("message_length: {}", message_length);

        // let mut res_message: Vec<u8> = vec![0; message_length];
        let mut res_message = [0; 68];
        let res_message_length = self.stream.read(&mut res_message).unwrap();

        // println!("res_message_length: {}", res_message_length);

        if res_message_length < 68 {
            panic!("During handshake the peer sent back less than 68 bytes. Maybe choose a different peer.");
        }

        let res_peer_id = &res_message[res_message_length - 20..];

        return res_peer_id.to_vec();
    }
}

#[derive(Debug)]
pub struct BlockRequestPayload {
    piece_index: u32,
    block_byte_offset: u32,
    block_length: u32,
}

impl BlockRequestPayload {
    fn to_bytes(&self) -> Vec<u8> {
        let mut block_request_payload_bytes: Vec<u8> = Vec::new();

        // converting piece index to bytes array
        let piece_index_buf = self.piece_index.to_be_bytes();
        // adding piece index bytes to the payload bytes array
        block_request_payload_bytes.extend(piece_index_buf);

        // converting block byte offset to bytes array
        let block_byte_offset_buf = self.block_byte_offset.to_be_bytes();
        // adding block byte offset bytes to the paybload bytes array
        block_request_payload_bytes.extend(block_byte_offset_buf);

        // converting block length to bytes array
        let block_length_buf = self.block_length.to_be_bytes();
        // adding block lenght bytes to the payload bytes array
        block_request_payload_bytes.extend(block_length_buf);

        return block_request_payload_bytes;
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BlockResponsePayload {
    piece_index: u32,
    block_byte_offset: u32,
    block_data: Vec<u8>,
}

impl BlockResponsePayload {
    fn from(payload: Vec<u8>, payload_length: u32) -> Result<Self, (String, Self)> {
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

        if block_data_length != payload_length - 8 {
            return Err((
                "Invalid block data length".to_string(),
                block_response_payload,
            ));
        }

        Ok(block_response_payload)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Piece {
    piece_index: u32,
    piece_length: u32,
    piece_data: Vec<u8>,
    piece_hash: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum PeerMessageType {
    BitField,
    Interested,
    Unchoke,
    Request,
    Piece,
}

#[allow(dead_code)]
impl PeerMessageType {
    pub fn from(message_id: u8) -> Self {
        match message_id {
            1 => Self::Unchoke,
            2 => Self::Interested,
            5 => Self::BitField,
            6 => Self::Request,
            7 => Self::Piece,
            _ => panic!("unknown message id {}", message_id),
        }
    }

    pub fn to_message_id(&self) -> u8 {
        match self {
            Self::Unchoke => 1,
            Self::Interested => 2,
            Self::BitField => 5,
            Self::Request => 6,
            Self::Piece => 7,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PeerMessage {
    length: u32,
    message_type: PeerMessageType,
    payload: Vec<u8>,
}

#[allow(dead_code)]
impl PeerMessage {
    fn wait(connection: &mut Connection, message_type: PeerMessageType) -> Self {
        let wait_message_types = vec![
            PeerMessageType::BitField,
            PeerMessageType::Unchoke,
            PeerMessageType::Piece,
        ];

        // check if can wait for given message_type
        if wait_message_types.contains(&message_type) {
            // read message length (4 Bytes)
            let mut recv_buf_length: [u8; 4] = [0; 4];
            connection
                .stream
                .read_exact(&mut recv_buf_length)
                .expect("PeerMessage::wait - Failed to read length prefix");
            // println!("buf_length: {:?}", buf_length);

            // message = message_id + payload
            let recv_message_length = u32::from_be_bytes(recv_buf_length);
            // println!("length: {}", message_length);

            // read message id (1 Byte)
            let mut recv_buf_message_id: [u8; 1] = [0; 1];
            connection
                .stream
                .read_exact(&mut recv_buf_message_id)
                .expect("PeerMessage::wait - Failed to read message id");
            // println!("buf_message_id: {:?}", buf_message_id);
            let recv_message_id = u8::from_be_bytes(recv_buf_message_id);
            // println!("message_id: {}", message_id);
            let recv_message_type = PeerMessageType::from(recv_message_id);
            // println!("message_type (enum): {:?}", message_type);

            if recv_message_type != message_type {
                panic!(
                    "PeerMessage::wait - message_id should be {:?}",
                    message_type
                );
            }

            // read payload
            let payload_length = (recv_message_length - 1) as usize;
            let mut recv_payload_buf: Vec<u8> = vec![0; payload_length];
            connection
                .stream
                .read_exact(&mut recv_payload_buf)
                .expect("PeerMessage::wait - Failed to read payload");
            // println!("buf_payload read of size: {}", recv_payload_buf.len());

            // return PeerMessage
            return Self {
                length: recv_message_length,
                message_type: recv_message_type,
                payload: recv_payload_buf,
            };
        }

        panic!(
            "PeerMessage::wait - Cannot create wait message for message_type {:?}",
            message_type
        );
    }

    fn send(connection: &mut Connection, message_type: PeerMessageType, payload: Vec<u8>) -> Self {
        let send_message_types = vec![PeerMessageType::Interested, PeerMessageType::Request];

        if send_message_types.contains(&message_type) {
            // bytes to send (message_id + payload)
            // adding message id to message_buf
            let message_id_byte = message_type.to_message_id();
            let mut message_buf: Vec<u8> = vec![message_id_byte];

            // adding payload to message_buf
            message_buf.extend(&payload);

            // calculating message_buf length
            let message_length = message_buf.len() as u32;
            let message_length_buf = message_length.to_be_bytes();

            // sending message length
            connection
                .stream
                .write_all(&message_length_buf)
                .expect("request message length could not be sent");
            // sending the message
            connection
                .stream
                .write_all(&message_buf)
                .expect("request message could not be sent");

            return Self {
                length: message_length,
                message_type,
                payload,
            };
        }

        panic!(
            "Cannot create send message for message_type {:?}",
            message_type
        );
    }

    fn download_block(
        connection: &mut Connection,
        block_request_payload: BlockRequestPayload,
    ) -> BlockResponsePayload {
        // Sending a `request` message
        Self::send(
            connection,
            PeerMessageType::Request,
            block_request_payload.to_bytes(),
        );

        // Waiting for a `piece` message
        let piece_peer_message = Self::wait(connection, PeerMessageType::Piece);

        BlockResponsePayload::from(piece_peer_message.payload, piece_peer_message.length - 1)
            .expect("Invalid block data received")
    }

    // fn send_block_request(connection: &mut Connection, block_request_payload: BlockRequestPayload) {
    //     Self::send(
    //         connection,
    //         PeerMessageType::Request,
    //         block_request_payload.to_bytes(),
    //     );
    // }

    // fn wait_for_block_response(connection: &mut Connection) -> BlockResponsePayload {
    //     let piece_peer_message = Self::wait(connection, PeerMessageType::Piece);

    //     BlockResponsePayload::from(piece_peer_message.payload, piece_peer_message.length - 1).expect("Invalid block data received")
    // }

    #[allow(unused_variables)]
    fn download_piece(connection: &mut Connection, piece_index: u32, piece_length: u32) -> Piece {
        // Send Peer messages

        // 1. Wait for `bitfield` message
        Self::wait(connection, PeerMessageType::BitField);

        // 2. Send an `interested` message
        Self::send(connection, PeerMessageType::Interested, Vec::new());

        // 3. Wait until you receive an `unchoke` message
        Self::wait(connection, PeerMessageType::Unchoke);

        // 4. Send a `request` message
        // let block_request_payload = BlockRequestPayload { piece_index, block_byte_offset: 0, block_length: 16 * 1024 };
        // Self::send_block_request(connection, block_request_payload);

        // 5. Wait for a `piece` message
        // let block_response_payload = Self::wait_for_block_response(connection);


        let mut block_index = 0 as u32;
        let mut block_byte_offset: u32;
        let block_length = 16 * 1024 as u32;
        let mut piece_data: Vec<u8> = Vec::new();

        loop {
            block_byte_offset = block_index * block_length;

            if block_byte_offset >= piece_length - 1 {
                break;
            }

            let actual_block_length = if piece_length - block_byte_offset < block_length {
                piece_length - block_byte_offset
            } else {
                block_length
            };

            println!(">> Downloading block offset {} of size {}", block_byte_offset, actual_block_length);

            // 4,5. Download a block
            let block_request_payload = BlockRequestPayload {
                piece_index,
                block_byte_offset,
                block_length: actual_block_length,
            };
            let block_response_payload = Self::download_block(connection, block_request_payload);

            // println!("block_response_payload: {:?}", block_response_payload);
            // println!(
            //     "block_response_payload: piece_index: {}, block_byte_offset: {}",
            //     block_response_payload.piece_index, block_response_payload.block_byte_offset
            // );
            // println!(
            //     "block_response_payload length: {}",
            //     block_response_payload.block_data.len()
            // );

            // append the block data to the piece data
            piece_data.extend(block_response_payload.block_data);

            block_index += 1;
        }

        // println!("piece_length: {}", piece_length);

        let piece_hash = calculate_hash(piece_data.clone());

        // println!("piece_hash: {:?}", piece_hash);

        Piece {
            piece_index,
            piece_length,
            piece_data,
            piece_hash,
        }
    }
}

fn calculate_hash(bytes: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    hash.to_vec()
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
