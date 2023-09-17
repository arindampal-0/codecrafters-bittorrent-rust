use clap::{Parser, Subcommand};

use crate::{
    decode::{DecodeArgs, self}, download_piece::{DownloadPieceArgs, self}, handshake::{HandshakeArgs, self},
    info::{InfoArgs, self}, peers::{PeersArgs, self},
};

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
    Decode(DecodeArgs),
    Info(InfoArgs),
    Peers(PeersArgs),
    Handshake(HandshakeArgs),
    #[clap(name = "download_piece")]
    DownloadPiece(DownloadPieceArgs),
}

pub async fn parse_and_execute() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Decode(args) => decode::execute(args),
        Command::Info(args) => info::execute(args),
        Command::Peers(args) => peers::execute(args).await,
        Command::Handshake(args) => handshake::execute(args),
        Command::DownloadPiece(args) => download_piece::execute(args),
    };
}