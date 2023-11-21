use std::path::PathBuf;

use anyhow::Context;
use bittorrent_starter_rust::{
    torrent::{self, decode_bencode_value, Torrent},
    tracker::{TrackerRequest, TrackerResponse},
};
use clap::{Parser, Subcommand};

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
    Peers { torrent: PathBuf },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

            let length = if let torrent::Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!()
            };
            println!("Length: {length}");

            let info_hash = t.info_hash();
            println!("Info Hash: {}", hex::encode(info_hash));

            // Piece length and piece Hashes
            println!("Piece Length: {}", t.info.plength);
            println!("Piece Haashes:");
            for piece in t.info.pieces.0 {
                println!("{}", hex::encode(piece));
            }
        }
        Commands::Peers { torrent } => {
            let dot_torrent = std::fs::read(torrent).context("read torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;

            let length = if let torrent::Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!()
            };

            let info_hash = t.info_hash();

            let request = TrackerRequest {
                peer_id: String::from("00112233445566778899"),
                port: 6881,
                uploaded: 0,
                downloaded: 0,
                left: length,
                compact: 1,
            };
            let url_params =
                serde_urlencoded::to_string(request).context("url-encode tracker parameters")?;
            let tracker_url = format!(
                "{}?{}&info_hash={}",
                t.announce,
                url_params,
                &urlencode(&info_hash)
            );

            // let mut tracker_url =
            //     reqwest::Url::parse(&t.announce).context("parse tracker announce URL")?;
            // tracker_url.set_query(Some(&url_params));

            let response = reqwest::get(tracker_url).await.context("query tracker")?;
            let response = response.bytes().await.context("fetch tracker response")?;
            let response: TrackerResponse =
                serde_bencode::from_bytes(&response).context("parse tracker response")?;

            for peer in response.peers.0 {
                println!("{}:{}", peer.ip(), peer.port());
            }
        }
    }

    Ok(())
}

fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::new();
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte][..]));
    }
    encoded
}
