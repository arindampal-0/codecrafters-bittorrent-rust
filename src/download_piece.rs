use std::cmp::Ordering;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::{fs, vec};

use clap::Args;

use crate::{Connection, PeerMessage, PeerMessageType, TorrentMetadata, TrackerResponse, to_hex_string};

#[allow(dead_code)]
#[derive(Args, Debug)]
pub struct DownloadPieceArgs {
    /// torrent file path
    torrent_file_path: PathBuf,
    /// output file path
    #[clap(short, long)]
    output: PathBuf,
    /// piece index
    piece: usize,
}

#[allow(unreachable_code)]
/// For command: "download_piece"
pub async fn execute(args: &DownloadPieceArgs) {
    // println!("args: {:?}", args);

    // Read torrent file to get tracker URL
    let torrent_metadata = TorrentMetadata::from_file(args.torrent_file_path.clone());

    // println!("info_hash: {}", torrent_metadata.info.hash_str());

    // Perform the tracker GET request to get list of peers
    let tracker_response = TrackerResponse::from(&torrent_metadata).await;

    let peers = tracker_response.get_peers();
    // peers.iter().for_each(|peer| {
    //     println!("{}", peer);
    // });

    let peer = peers.get(1).expect("There is no peer at that index");
    // println!("peer_addr: {}", peer);

    // Establish a TCP connection with a peer, and
    let mut connection = Connection::new(peer.clone());

    // Perform a handshake
    // let peer_id =
    connection.handshake(
        torrent_metadata.info.hash_bytes().to_vec(),
        "00112233445566778899",
    );
    // println!("peer_id: {}", to_hex_string(&peer_id));

    let piece_index = args.piece as u32;
    println!("piece_index: {}", piece_index);

    let pieces_count = torrent_metadata.info.pieces.chunks(20).len() as u32;
    println!("pieces_count: {}", pieces_count);

    assert!(
        piece_index < pieces_count,
        "This piece does not exists, max pieces: {}",
        pieces_count
    );

    let piece_length = torrent_metadata.info.piece_length;
    println!("piece_length: {}", piece_length);

    let torrent_file_length = torrent_metadata
        .info
        .length
        .expect("Torrent file length was not present");
    println!("file_length: {}", torrent_file_length);


    let is_last_piece = torrent_file_length - piece_index * piece_length < piece_length;
    let actual_piece_length = if is_last_piece {
        torrent_file_length - piece_index * piece_length
    } else {
        piece_length
    };

    println!("actual_piece_length: {}", actual_piece_length);

    let piece = PeerMessage::download_piece(&mut connection, piece_index, actual_piece_length);

    let piece_hashes = torrent_metadata.info.get_piece_hashes();
    let actual_piece_hash = piece_hashes
        .get(args.piece)
        .expect("Could not get first piece hash from torrent info");

    // println!("actual piece_hash: {:?}", actual_piece_hash);

    // if to_hex_string(&actual_piece_hash) == to_hex_string(&piece.piece_hash) {
    //     println!("String comparision - Piece hash matches");
    // }

    if actual_piece_hash
        .clone()
        .into_iter()
        .cmp(piece.piece_hash.into_iter())
        == Ordering::Equal
    {
        // println!("Both hashes are equal");

        // Create the directories if not exists
        let parent_directory = args
            .output
            .parent()
            .expect("Failed to extract parent directory of output file");
        fs::create_dir_all(parent_directory)
            .expect("Failed to recursively create parent directories");

        // Create file if not exists
        let mut file = fs::File::create(args.output.clone()).expect(
            format!(
                "Could not create file {}",
                args.output
                    .to_str()
                    .expect("Failed Pathbuf to str conversion")
            )
            .as_str(),
        );

        // write the piece to the output file
        file.write_all(&piece.piece_data)
            .expect("Could not write the piece to file");

        println!(
            "Piece {} downloaded to {}",
            args.piece,
            args.output
                .to_str()
                .expect("Failed Pathbuf to str conversion")
        );
    }

    return;

    // Exchange multiple peer messages to download the piece
    // 1. Wait for a `bitfield`
    let _bitfield_message = PeerMessage {
        length: 0,
        message_type: PeerMessageType::BitField,
        payload: Vec::new(),
    };

    println!("{}", "[wait for bitfield]".to_uppercase());

    // read message length (4 Bytes)
    let mut buf_length: [u8; 4] = [0; 4];
    connection
        .stream
        .read_exact(&mut buf_length)
        .expect("Failed to read length prefix");
    println!("buf_length: {:?}", buf_length);

    let total_length = u32::from_be_bytes(buf_length);
    println!("length: {}", total_length);

    // read message id (1 Byte)
    let mut buf_message_id: [u8; 1] = [0; 1];
    connection
        .stream
        .read_exact(&mut buf_message_id)
        .expect("Failed to read message id");
    println!("buf_message_id: {:?}", buf_message_id);
    let message_id = u8::from_be_bytes(buf_message_id);
    println!("message_id: {}", message_id);
    let message_id = PeerMessageType::from(message_id);
    println!("message_id (enum): {:?}", message_id);

    if message_id != PeerMessageType::BitField {
        panic!("message_id should be {:?}", PeerMessageType::BitField);
    }

    // read payload
    let payload_length = (total_length - 1) as usize;
    let mut buf_payload: Vec<u8> = vec![0; payload_length];
    connection
        .stream
        .read_exact(&mut buf_payload)
        .expect("Failed to read payload");
    println!("buf_payload read of size: {}", buf_payload.len());

    // 2. Send an `interested` message
    println!("{}", "[Send an interested message]".to_uppercase());
    // bytes to send (message_id + payload)
    let message: Vec<u8> = vec![2];

    // length of the message
    let length = message.len() as u32;
    let length_buf: [u8; 4] = length.to_be_bytes();

    // sending length
    connection.stream.write(&length_buf).unwrap();
    // sending the message
    connection.stream.write(&message).unwrap();

    // 3. wait until you get `unchoke` message
    println!("{}", "[Wait for unchoke]".to_uppercase());
    // Read message length
    let mut length_buf: [u8; 4] = [0; 4];
    connection.stream.read(&mut length_buf).expect(
        "
    Failed to read length prefix",
    );
    let total_length = u32::from_be_bytes(length_buf);
    println!("total_length: {}", total_length);

    let mut message_id_buf: [u8; 1] = [0; 1];
    connection
        .stream
        .read(&mut message_id_buf)
        .expect("Failed to read message id");
    let message_id = u8::from_be_bytes(message_id_buf);
    let message_id = PeerMessageType::from(message_id);
    println!("message_id: {:?}", message_id);

    if message_id != PeerMessageType::Unchoke {
        panic!("message id should be {:?}", PeerMessageType::Unchoke);
    }

    // 4. Send a `request` message
    println!("{}", "[Send a request]".to_uppercase());
    // let piece_length = torrent_metadata.info.piece_length as u32;
    let piece_index = args.piece as u32;

    const BLOCK_SIZE: u32 = 16 * 1024;
    // FIXME: add ceil division
    // let blocks_count = piece_length / BLOCK_SIZE;

    let block_length = BLOCK_SIZE;

    // adding message id
    let mut message_buf: Vec<u8> = vec![6];
    // adding payload: piece index (index)
    let piece_index_buf = piece_index.to_be_bytes();
    message_buf.extend(piece_index_buf.to_vec());
    // adding payload : block offset (begin)
    let block_offset: u32 = 0;
    let block_offset_buf = block_offset.to_be_bytes();
    message_buf.extend(block_offset_buf.to_vec());
    // adding payload: block length (length)
    let block_length_buf = block_length.to_be_bytes();
    message_buf.extend(block_length_buf.to_vec());

    let total_length = message_buf.len() as u32;
    let total_length_buf = total_length.to_be_bytes();

    // sending message length
    connection
        .stream
        .write(&total_length_buf)
        .expect("request message length could not be sent");
    // sending the message
    connection
        .stream
        .write(&message_buf)
        .expect("request message could not be sent");

    // 5. Wait for a `piece` message
    println!("{}", "[Wait for a piece]".to_uppercase());

    // Read message length
    let mut length_buf: [u8; 4] = [0; 4];
    connection.stream.read(&mut length_buf).expect(
        "
    Failed to read length prefix",
    );
    let total_length = u32::from_be_bytes(length_buf);
    println!("total_length: {}", total_length);

    let mut message_id_buf: [u8; 1] = [0; 1];
    connection
        .stream
        .read(&mut message_id_buf)
        .expect("Failed to read message id");
    let message_id = u8::from_be_bytes(message_id_buf);
    let message_id = PeerMessageType::from(message_id);
    println!("message_id: {:?}", message_id);

    if message_id != PeerMessageType::Piece {
        panic!("message id should be {:?}", PeerMessageType::Piece);
    }

    // read piece index
    let mut res_piece_index_buf: [u8; 4] = [0; 4];
    connection
        .stream
        .read_exact(&mut res_piece_index_buf)
        .expect("Could not read piece index from piece message");
    let res_piece_index = u32::from_be_bytes(res_piece_index_buf);
    println!("res_piece_index: {}", res_piece_index);

    // read block offset (in bytes)
    let mut res_block_offset_buf: [u8; 4] = [0; 4];
    connection
        .stream
        .read_exact(&mut res_block_offset_buf)
        .expect("Could not read block offset from piece message");
    let res_block_offset = u32::from_be_bytes(res_block_offset_buf);
    println!("res_block_offset: {}", res_block_offset);

    // read the block data
    let block_data_length = total_length - 9;
    let mut block_data: Vec<u8> = vec![0; block_data_length as usize];
    connection
        .stream
        .read(&mut block_data)
        .expect("Could not read block data from piece message");

    println!("block data length: {}", block_data.len());
}
