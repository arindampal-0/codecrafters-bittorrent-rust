use std::path::PathBuf;

use crate::{TorrentMetadata, TrackerResponse};
use clap::Args;

#[derive(Args, Debug)]
pub struct PeersArgs {
    /// torrent file path
    torrent_file_path: PathBuf,
}

/// For command: "peers"
pub async fn execute(args: &PeersArgs) {
    let torrent_metadata = TorrentMetadata::from_file(args.torrent_file_path.clone());

    let tracker_response = TrackerResponse::from(&torrent_metadata).await;

    tracker_response
        .get_peers()
        .iter()
        .for_each(|peer_str| println!("{}", peer_str));
}
