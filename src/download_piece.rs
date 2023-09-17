use std::path::PathBuf;

use clap::Args;

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

/// For command: "download_piece"
pub fn execute(args: &DownloadPieceArgs) {
    println!("args: {:?}", args);
}
