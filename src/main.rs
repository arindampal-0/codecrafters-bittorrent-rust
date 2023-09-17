use bittorrent_starter_rust::cli;
// use std::env;

#[tokio::main]
async fn main() {
    // let args: Vec<String> = env::args().collect();
    // println!("{:?}", args);
    // let command = args.get(1).expect("No command specified");
    // let command = &args[1];

    // if command == "decode" {
    //     let encoded_value = &args[2];
    //     // decode(encoded_value);
    // } else if command == "info" {
    //     let torrent_file_path = &args[2];
    //     // read_torrent_metadata(torrent_file_path);
    // } else if command == "peers" {
    //     let torrent_file_path = &args[2];
    //     // get_torrent_peers(torrent_file_path).await?;
    // } else if command == "handshake" {
    //     let torrent_file_path = &args[2];
    //     let peer_address = &args[3];
    //     // peer_handshake(torrent_file_path, peer_address);
    // } else if command == "download_piece" {
    //     let torrent_file_path = &args[2];
    // } else {
    //     println!("unknown command: {}", args[1]);
    // }

    cli::parse_and_execute().await;
}
