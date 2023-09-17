use std::path::PathBuf;

use crate::{urlencode_hash, TorrentMetadata, TrackerResponse};
use clap::Args;

#[derive(Args, Debug)]
pub struct PeersArgs {
    /// torrent file path
    torrent_file_path: PathBuf,
}

/// For command: "peers"
pub async fn execute(args: &PeersArgs) {
    let torrent_metadata = TorrentMetadata::from_file(args.torrent_file_path.clone());

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

    tracker_response
        .print_peers()
        .iter()
        .for_each(|peer_str| println!("{}", peer_str));
}
