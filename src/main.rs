use std::path::PathBuf;

use anyhow::Context;
use bittorrent_starter_rust::{
    torrent::{Keys, Torrent},
    tracker::TrackerRequest,
};
use clap::{Parser, Subcommand};
use serde_bencode;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Decode { value: String },
    Info { torrent: PathBuf },
    Peers { torrent: PathBuf },
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Decode { value } => {
            // let v: serde_json::Value = serde_bencode::from_str(&value).unwrap();
            // println!("{}", v);
            unimplemented!("serde_bencode -> serde_json::Value is borked");
        }
        Command::Info { torrent } => {
            let dot_torrent = std::fs::read(torrent).context("open torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;

            println!("Tracker URL: {}", t.announce);
            let length = if let Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!();
            };
            println!("Length: {}", length);

            let info_hash = t.info_hash();
            println!("Info Hash: {}", hex::encode(info_hash));
            println!("Piece Hashes: {}", t.info.plength);
            println!("Piece Hashes:");
            for hash in t.info.pieces.0 {
                println!("{}", hex::encode(hash));
            }
        }
        Command::Peers { torrent } => {
            let dot_torrent = std::fs::read(torrent).context("open torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;

            let length = if let Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!();
            };

            let info_hash = t.info_hash();
            let request = TrackerRequest {
                info_hash,
                peer_id: String::from("00112233445566778899"),
                port: 6881,
                uploaded: 0,
                downloaded: 0,
                left: length,
                compact: 0,
            };

            let mut tracker_url = reqwest::Url::parse(&t.announce).context("parse url")?;
            let url_params =
                serde_urlencoded::to_string(&request).context("url-encode tracker parameters")?;
            tracker_url.set_query(Some(&url_params));

            let response = reqwest::get(tracker_url).await.context("fetch tracker");
            println!("{response:?}");
        }
    }

    Ok(())
}
