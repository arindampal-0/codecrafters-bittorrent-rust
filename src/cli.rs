use clap::{Parser, Subcommand};

use crate::{decode, download, download_piece, handshake, info, peers};

#[derive(Parser, Debug)]
#[clap(
    author = "Arindam Pal",
    name = "bittorrent-client",
    version,
    about = "Simple bittorrent client (I guess 😋)"
)]
struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Decode(decode::DecodeArgs),
    Info(info::InfoArgs),
    Peers(peers::PeersArgs),
    Handshake(handshake::HandshakeArgs),
    #[clap(name = "download_piece")]
    DownloadPiece(download_piece::DownloadPieceArgs),
    Download(download::DownloadArgs),
}

pub async fn parse_and_execute() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Decode(args) => decode::execute(args),
        Command::Info(args) => info::execute(args),
        Command::Peers(args) => peers::execute(args).await,
        Command::Handshake(args) => handshake::execute(args),
        Command::DownloadPiece(args) => download_piece::execute(args).await,
        Command::Download(args) => download::execute(args).await,
    };
}
