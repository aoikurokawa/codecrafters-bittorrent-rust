use std::path::PathBuf;

use anyhow::Context;
use bittorrent_starter_rust::torrent::{decode_bencode_value, Keys, Torrent};
use clap::{Parser, Subcommand};
use sha1::{Digest, Sha1};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Decode { value: String },
    Info { torrent: PathBuf },
}

fn main() -> anyhow::Result<()> {
    // let args: Vec<String> = env::args().collect();
    let args = Args::parse();

    match args.command {
        Commands::Decode { value } => {
            let v = decode_bencode_value(&value).0;
            println!("{v}");
        }
        Commands::Info { torrent } => {
            let dot_torrent = std::fs::read(torrent).context("read torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;
            println!("Tracker URL: {}", t.announce);

            if let Keys::SingleFile { length } = t.info.keys {
                println!("Length: {}", length);
            } else {
                todo!()
            }

            // Info hash
            let info_encoded =
                serde_bencode::to_bytes(&t.info).context("re-encode info section")?;
            let mut hasher = Sha1::new();
            hasher.update(&info_encoded);
            let info_hash = hasher.finalize();
            println!("Info Hash: {}", hex::encode(info_hash));

            // Piece length and piece Hashes
            println!("Piece Length: {}", t.info.plength);
            println!("Piece Haashes:");
            for piece in t.info.pieces.0 {
                println!("{}", hex::encode(piece));
            }

            // peer_id: You can use something like 00112233445566778899.
            // port: You can set this to 6881,
            // For the purposes of this challenge, set this to 1.
        }
    }

    Ok(())
}
