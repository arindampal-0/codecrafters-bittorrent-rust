use clap::Args;

use crate::{to_hex_string, Connection, TorrentMetadata};
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

    let mut connection = Connection::new(args.peer_address.clone());
    let res_peer_id = connection.handshake(
        torrent_metadata.info.hash_bytes().to_vec(),
        "00112233445566778899",
    );

    println!("Peer ID: {}", to_hex_string(&res_peer_id));
}
