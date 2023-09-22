use std::path::PathBuf;

use clap::Args;

use crate::TorrentMetadata;

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// torrent file path
    torrent_file_path: PathBuf,
}

/// For command: "info"
pub fn execute(args: &InfoArgs) {
    let torrent_metadata = TorrentMetadata::from_file(args.torrent_file_path.clone());

    println!("Tracker URL: {}", torrent_metadata.announce);
    // println!("Info: {:?}", torrent_metadata.info);
    println!("Length: {}", torrent_metadata.info.length.unwrap());

    println!("Info Hash: {}", torrent_metadata.info.hash_str());

    println!("Piece Length: {}", torrent_metadata.info.piece_length);

    println!("Piece Hashes:");
    torrent_metadata
        .info
        .get_piece_hashes_str()
        .iter()
        .for_each(|piece_hash_str| {
            println!("{}", piece_hash_str);
        });
}
