use bittorrent_starter_rust::cli;

#[tokio::main]
async fn main() {
    cli::parse_and_execute().await;
}


// #[allow(dead_code)]
// async fn download_piece_try() {
//     // TRYOUT download_piece

//     // Read torrent file to get tracker URL
//     let torrent_metadata = TorrentMetadata::from_file(PathBuf::from(
//         // "/home/arindampal/Downloads/congratulations.gif.torrent",
//         "sample.torrent",
//     ));

//     // println!("info_hash: {}", torrent_metadata.info.hash_str());

//     // Perform the tracker GET request to get list of peers
//     let tracker_response = TrackerResponse::from(&torrent_metadata).await;

//     let peers = tracker_response.get_peers();
//     // peers.iter().for_each(|peer| {
//     //     println!("{}", peer);
//     // });

//     let peer = peers.get(1).expect("There is no peer at that index");
//     println!("peer_addr: {}", peer);

//     // Establish a TCP connection with a peer, and
//     let mut connection = Connection::new(peer.clone());

//     // Perform a handshake
//     // let peer_id =
//     connection.handshake(
//         torrent_metadata.info.hash_bytes().to_vec(),
//         "00112233445566778899",
//     );
//     // println!("peer_id: {}", to_hex_string(&peer_id));

//     let piece_index = 0 as u32;
//     println!("piece_index: {}", piece_index);

//     let pieces_count = torrent_metadata.info.pieces.chunks(20).len() as u32;
//     println!("pieces_count: {}", pieces_count);

//     // check piece_index if valid
//     if piece_index >= pieces_count {
//         return;
//     }

//     let piece_length = torrent_metadata.info.piece_length;
//     println!("piece_lenght: {}", piece_length);

//     let file_length = torrent_metadata
//         .info
//         .length
//         .expect("Torrent file length was not present");
//     println!("file_length: {}", file_length);

//     let actual_piece_length = if piece_index == pieces_count - 1 {
//         // (torrent.length() - (torrent.info.piece_length as u64 * piece_id as u64)) as u32
//         file_length - (piece_length * piece_index)
//     } else {
//         piece_length
//     };

//     println!("actual_piece_length: {}", actual_piece_length);

//     let send_message_types = vec![PeerMessageType::Interested, PeerMessageType::Request];

//     let message_type = PeerMessageType::Interested;
//     if send_message_types.contains(&message_type) {
//         println!("here");
//     }

//     // Peer Messages
//     // 1. Wait for a `bitfield` message
//     wait(&mut connection, PeerMessageType::BitField);
//     println!("WAIT `bitfield`");

//     // 2. Send an `interested` message
//     let interested_payload: Vec<u8> = vec![2];
//     send(
//         &mut connection,
//         PeerMessageType::Interested,
//         interested_payload,
//     );
//     println!("SEND `interested`");

//     // 3. Wait for `unchoke`
//     wait(&mut connection, PeerMessageType::Unchoke);
//     println!("WAIT `unchoke`");

//     // 4. send piece `request`
//     let block_index = 0 as u32;
//     let block_length = 16 * 1024 as u32;

//     let mut request_payload: Vec<u8> = Vec::new();
//     request_payload.extend(piece_index.to_be_bytes());
//     request_payload.extend(block_index.to_be_bytes());
//     request_payload.extend(block_length.to_be_bytes());

//     send(&mut connection, PeerMessageType::Request, request_payload);
//     println!("SEND `request`");

//     // 5. wait for `piece` message
//     let (_, block_buf) = wait(&mut connection, PeerMessageType::Piece);
//     println!("block_buf length: {}", block_buf.len());
// }

// fn wait(connection: &mut Connection, message_type: PeerMessageType) -> (u32, Vec<u8>) {
//     let wait_message_types = vec![
//         PeerMessageType::BitField,
//         PeerMessageType::Unchoke,
//         PeerMessageType::Piece,
//     ];

//     // check if can wait for given message_type
//     if wait_message_types.contains(&message_type) {
//         // read message length (4 Bytes)
//         let mut recv_buf_length: [u8; 4] = [0; 4];
//         connection
//             .stream
//             .read_exact(&mut recv_buf_length)
//             .expect("PeerMessage::wait - Failed to read length prefix");
//         // println!("buf_length: {:?}", buf_length);

//         // message = message_id + payload
//         let recv_message_length = u32::from_be_bytes(recv_buf_length);
//         println!("length: {}", recv_message_length);

//         // read message id (1 Byte)
//         let mut recv_buf_message_id: [u8; 1] = [0; 1];
//         connection
//             .stream
//             .read_exact(&mut recv_buf_message_id)
//             .expect("PeerMessage::wait - Failed to read message id");
//         // println!("buf_message_id: {:?}", buf_message_id);
//         let recv_message_id = u8::from_be_bytes(recv_buf_message_id);
//         // println!("message_id: {}", message_id);
//         let recv_message_type = PeerMessageType::from(recv_message_id);
//         // println!("message_type (enum): {:?}", message_type);

//         if recv_message_type != message_type {
//             panic!(
//                 "PeerMessage::wait - message_id should be {:?}",
//                 message_type
//             );
//         }

//         // read payload
//         let payload_length = (recv_message_length - 1) as usize;
//         let mut recv_payload_buf: Vec<u8> = vec![0; payload_length];
//         connection
//             .stream
//             .read_exact(&mut recv_payload_buf)
//             .expect("PeerMessage::wait - Failed to read payload");
//         // println!("buf_payload read of size: {}", recv_payload_buf.len());

//         // return PeerMessage
//         return (recv_message_length, recv_payload_buf);
//     }

//     panic!(
//         "PeerMessage::wait - Cannot create wait message for message_type {:?}",
//         message_type
//     );
// }

// fn send(connection: &mut Connection, message_type: PeerMessageType, payload: Vec<u8>) {
//     let send_message_types = vec![PeerMessageType::Interested, PeerMessageType::Request];

//     if send_message_types.contains(&message_type) {
//         // bytes to send (message_id + payload)
//         // adding message id to message_buf
//         let message_id_byte = message_type.to_message_id();
//         let mut message_buf: Vec<u8> = vec![message_id_byte];

//         // adding payload to message_buf
//         message_buf.extend(&payload);

//         // calculating message_buf length
//         let message_length = message_buf.len() as u32;
//         let message_length_buf = message_length.to_be_bytes();

//         // sending message length
//         connection
//             .stream
//             .write_all(&message_length_buf)
//             .expect("request message length could not be sent");
//         // sending the message
//         connection
//             .stream
//             .write_all(&message_buf)
//             .expect("request message could not be sent");

//         return;
//     }

//     panic!(
//         "Cannot create send message for message_type {:?}",
//         message_type
//     );
// }
