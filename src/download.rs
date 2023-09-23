use std::path::PathBuf;
use std::{fs, os::unix::prelude::FileExt};

use clap::Args;

use crate::{to_hex_string, Connection, PeerMessageType, TorrentMetadata, TrackerResponse};

#[derive(Args, Debug)]
pub struct DownloadArgs {
    /// .torrent file path
    torrent_file_path: PathBuf,
    /// output file path
    #[clap(short, long)]
    output: PathBuf,
}

pub async fn execute(args: &DownloadArgs) {
    // println!("args: {:?}", args);

    // Get torrent metadata
    let torrent_metadata = TorrentMetadata::from_file(args.torrent_file_path.clone());

    // get torrent tracker
    let tracker_response = TrackerResponse::from(&torrent_metadata).await;

    // get peers
    let peers = tracker_response.get_peers();

    // choose a peer
    let peer = peers.get(1).expect("Could not fetch peer at that index");

    // Setup connection with the peer
    let mut connection = Connection::new(peer.clone());

    let peer_id = "00112233445566778899".to_string();

    let info_hash = torrent_metadata.info.hash_bytes().to_vec();

    let torrent_file_length = torrent_metadata
        .info
        .length
        .expect("Could not get length of torrent file");
    println!("torrent_file_length: {}", torrent_file_length);

    let pieces_count = torrent_metadata.info.get_pieces_count() as u32;
    println!("pieces_count: {}", pieces_count);

    let piece_length = torrent_metadata.info.piece_length as u32;
    println!("piece_length: {}", piece_length);

    // Perform handshake
    let res_peer_id = connection.handshake(info_hash, peer_id);
    println!("res_peer_id: {}", to_hex_string(&res_peer_id));

    // Send and wait for peer messages
    // 1. Wait for `bitfield`
    connection.wait(PeerMessageType::BitField);

    // 2. Send an `interested` message
    connection.send(PeerMessageType::Interested, vec![]);

    // 3. Wait until `unchoke` is received
    connection.wait(PeerMessageType::Unchoke);

    // get piece hashes
    let piece_hashes_str = torrent_metadata.info.get_piece_hashes_str();

    // Get parent directory of the output file
    let parent_directory = args
        .output
        .parent()
        .expect("Failed to extract parent directory of the output file");
    // Create the parent directories recursively if not exists
    fs::create_dir_all(parent_directory).expect("Failed to recursively create parent directories");

    // Create the output file if not exists
    let file = fs::File::create(args.output.clone()).expect(
        format!(
            "Could not create file {}",
            args.output
                .to_str()
                .expect("Failed Pathbuf to str conversion")
        )
        .as_str(),
    );

    // download each piece
    for piece_index in 0..pieces_count {
        println!(
            "> Downloading piece {} (total {} pieces)",
            piece_index, pieces_count
        );

        // calculate actual piece length
        let is_last_piece = torrent_file_length - piece_index * piece_length < piece_length;
        let actual_piece_length = if is_last_piece {
            torrent_file_length - piece_index * piece_length
        } else {
            piece_length
        };
        println!("actual_piece_length: {}", actual_piece_length);

        // download the piece
        let piece = connection.download_piece_pipelined(5, piece_index, actual_piece_length);

        // verify piece hash
        let actual_piece_hash_str = piece_hashes_str
            .get(piece_index as usize)
            .expect(format!("Could not get piece hash at index {}", piece_index).as_str())
            .clone();

        let calculated_piece_hash = piece.get_hash();
        let calculated_piece_hash_str = to_hex_string(&calculated_piece_hash);

        assert_eq!(
            calculated_piece_hash_str, actual_piece_hash_str,
            "Piece {} hash does not match, expected {:?} but calculated {:?}",
            piece_index, actual_piece_hash_str, calculated_piece_hash_str
        );

        // write the piece to file
        file.write_all_at(&piece.piece_data, (piece_index * piece_length).into())
            .expect(
                format!(
                    "Could not write piece {} to the file at offset {}",
                    piece_index,
                    piece_index * piece_length
                )
                .as_str(),
            );
        println!("Piece {} written to file at offset {}", piece_index, piece_index * piece_length);
    }

    println!(
        "Downloaded {} to {}",
        args.torrent_file_path
            .to_str()
            .expect("Failed PathBuf to str conversion for args.torrent_file_path"),
        args.output
            .to_str()
            .expect("Failed PathBuf to str conversion for args.output")
    );
}
