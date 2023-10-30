mod hashes;

use std::{fs::File, path::PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use hashes::Hashes;
use serde::Deserialize;
use serde_bencode;
use serde_json;

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
}

/// A Metainfo file (also known as .torrent files).
#[derive(Debug, Clone, Deserialize)]
struct Torrent {
    /// The URL of the tracker.
    announce: String,

    /// This maps to a dictionary, with keys described below.
    info: Info,
}

#[derive(Debug, Clone, Deserialize)]
struct Info {
    /// The suggested name to save the file (or directory) as. It is purely advisory.
    name: String,

    /// The number of bytes in each piece the file is split into.
    ///
    /// For the purpose of transfer, files are split into fixed-size pieces which are all the same
    /// length excet for possibly the last one which may be truncated. piece length is almost
    /// always a power of two, most commonly 2^18 = 256K (BitTorrent prior to version 3.2 uses 2^20
    /// = 1M as default).
    #[serde(rename = "piece length")]
    plength: usize,

    /// Each entry of `pieces` is the SHA1 hash of the piece at the corresponding index.
    pieces: Hashes,

    #[serde(flatten)]
    keys: Keys,
}

/// There is a key `length` or a key `files`, but not both or neither.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Keys {
    /// If `length` is present then the download represents a signle file.
    ///
    /// In the single file case, the name key is the name of a file, in the multiple file case,
    /// it's the name of a directory.
    SingleFile {
        /// The length of the file in bytes.
        length: usize,
    },

    /// Otherwise it represents a set of files which go in a directory structure.
    ///
    /// For the purpose of the other keys in `Info`, the multi-file case is treated as only having a single
    /// file by concatenating the files in the order they appear in the files list.
    MultiFile {
        /// The files list is the value files maps to, and is a list of dictionaries containing the following keys:
        files: Vec<TorrentFile>,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct TorrentFile {
    /// The length of the file, in bytes
    length: usize,

    /// Subdirectory names for this file, the last of which is the actual file name
    /// (a zero length list is an error case).
    path: Vec<String>,
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Decode { value } => {
            // let v: serde_json::Value = serde_bencode::from_str(&value).unwrap();
            // println!("{}", v);
            unimplemented!("serde_bencode -> serde_json::Value is borked");
        }
        Command::Info { torrent } => {
            let mut dot_torrent = std::fs::read(torrent).context("open torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;

            println!("Tracker URL: {}", t.announce);
            if let Keys::SingleFile { length } = t.info.keys {
                println!("Length: {}", length);
            } else {
                todo!();
            }
        }
    }

    Ok(())
}
