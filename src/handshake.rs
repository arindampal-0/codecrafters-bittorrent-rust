use clap::Args;

use crate::{to_hex_string, TorrentMetadata};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct HandshakeArgs {
    /// torrent file path
    torrent_file_path: PathBuf,
    /// peer address (String) <IP_ADDR>:<PORT>
    peer_address: String,
}

/// For command: "handshake"
pub fn execute(args: &HandshakeArgs) {
    let torrent_metadata = TorrentMetadata::from_file(args.torrent_file_path.clone());
    let mut stream = TcpStream::connect(args.peer_address.clone()).expect("tcp connection failed!");

    // Length of the protocol string (1 Byte)
    let mut message: Vec<u8> = vec![19];
    // protocol string (19 Bytes)
    message.extend(b"BitTorrent protocol");
    // eight reserved bytes, all zeros (8 Bytes)
    message.extend(&[0; 8]);
    // sha1 info_hash (20 Bytes)
    message.extend(torrent_metadata.info.hash_bytes());
    // peer id (20 Bytes)
    message.extend(b"00112233445566778899");

    let message_length = stream.write(&message).unwrap();

    let mut res_message: Vec<u8> = vec![0; message_length];
    let res_message_length = stream.read(&mut res_message).unwrap();

    let res_peer_id = &res_message[res_message_length - 20..];

    println!("Peer ID: {}", to_hex_string(res_peer_id));
}
