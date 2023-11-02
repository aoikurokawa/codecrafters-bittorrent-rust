use std::{net::SocketAddrV4, path::PathBuf};

use anyhow::Context;
use bittorrent_starter_rust::{
    peer::{Handshake, Message, MessageFramer, MessageTag},
    torrent::{Keys, Torrent},
    tracker::{TrackerRequest, TrackerResponse},
};
use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use serde_bencode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PIECE_MAX: usize = 1 << 14;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[clap(rename_all = "snake_case")]
enum Command {
    Decode {
        value: String,
    },
    Info {
        torrent: PathBuf,
    },
    Peers {
        torrent: PathBuf,
    },
    Handshake {
        torrent: PathBuf,
        peer: String,
    },
    DownloadPiece {
        #[arg(short)]
        output: PathBuf,
        torrent: PathBuf,
        piece: usize,
    },
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Decode { value: _ } => {
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
                peer_id: String::from("00112233445566778899"),
                port: 6881,
                uploaded: 0,
                downloaded: 0,
                left: length,
                compact: 1,
            };

            let url_params =
                serde_urlencoded::to_string(&request).context("url-encode tracker parameters")?;
            let tracker_url = format!(
                "{}?{}&info_hash={}",
                t.announce,
                url_params,
                &urlencode(&info_hash)
            );

            let response = reqwest::get(tracker_url).await.context("query tracker")?;
            let response = response.bytes().await.context("fetch tracker response")?;
            let response: TrackerResponse =
                serde_bencode::from_bytes(&response).context("parse tracker response")?;

            for peer in &response.peers.0 {
                println!("{}:{}", peer.ip(), peer.port());
            }
        }
        Command::Handshake { torrent, peer } => {
            let dot_torrent = std::fs::read(torrent).context("open torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;

            let info_hash = t.info_hash();

            let peer = peer.parse::<SocketAddrV4>().context("parse peer address")?;
            let mut peer = tokio::net::TcpStream::connect(peer)
                .await
                .context("connect to peer")?;
            let mut handshake = Handshake::new(info_hash, *b"00112233445566778899");
            {
                let handshake_bytes =
                    &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];
                let handshake_bytes: &mut [u8; std::mem::size_of::<Handshake>()] =
                    unsafe { &mut *handshake_bytes };

                peer.write_all(handshake_bytes)
                    .await
                    .context("write handshake")?;

                peer.read_exact(handshake_bytes)
                    .await
                    .context("read handshake")?;
            }

            assert_eq!(handshake.length, 19);
            assert_eq!(&handshake.bittorent, b"BitTorrent protocol");

            println!("Peer ID: {}", hex::encode(&handshake.peer_id));
        }
        Command::DownloadPiece {
            output,
            torrent,
            piece,
        } => {
            let dot_torrent = std::fs::read(torrent).context("open torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;

            let length = if let Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!();
            };
            assert_eq!(piece, t.info.pieces.0.len());

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
                serde_urlencoded::to_string(&request).context("url-encode tracker parameters")?;
            let tracker_url = format!(
                "{}?{}&info_hash={}",
                t.announce,
                url_params,
                &urlencode(&info_hash)
            );

            let response = reqwest::get(tracker_url).await.context("query tracker")?;
            let response = response.bytes().await.context("fetch tracker response")?;
            let tracker_info: TrackerResponse =
                serde_bencode::from_bytes(&response).context("parse tracker response")?;

            let peer = &tracker_info.peers.0[0];
            let mut peer = tokio::net::TcpStream::connect(peer)
                .await
                .context("connect to peer")?;
            let mut handshake = Handshake::new(info_hash, *b"00112233445566778899");
            {
                let handshake_bytes =
                    &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];
                let handshake_bytes: &mut [u8; std::mem::size_of::<Handshake>()] =
                    unsafe { &mut *handshake_bytes };

                peer.write_all(handshake_bytes)
                    .await
                    .context("write handshake")?;

                peer.read_exact(handshake_bytes)
                    .await
                    .context("read handshake")?;
            }

            assert_eq!(handshake.length, 19);
            assert_eq!(&handshake.bittorent, b"BitTorrent protocol");

            println!("Peer ID: {}", hex::encode(&handshake.peer_id));

            let mut peer = tokio_util::codec::Framed::new(peer, MessageFramer);
            let bitfield = peer
                .next()
                .await
                .expect("peer always send a bitfield")
                .context("peer message was invalid")?;
            assert_eq!(bitfield.tag, MessageTag::Bitfield);
            // NOTE: we assume that the bitfield covers all pieces.

            peer.send(Message {
                tag: MessageTag::Interested,
                payload: Vec::new(),
            })
            .await
            .context("send interested message")?;

            let unchoke = peer
                .next()
                .await
                .expect("peer always send an unchoke")
                .context("peer message was invalid")?;
            assert_eq!(unchoke.tag, MessageTag::Unchoke);
            assert!(unchoke.payload.is_empty());

            let piece_hash = t.info.pieces.0[piece];
            let piece_size = if piece == t.info.pieces.0.len() + 1 {
                length % PIECE_MAX
            } else {
                PIECE_MAX
            };
        }
    }

    Ok(())
}

fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}
