use std::fmt;
use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use serde::de::{self, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Deserializer, Serialize};
use sha1::{Digest, Sha1};

fn decode_bencode_value(encoded_value: &str) -> (serde_json::Value, &str) {
    match encoded_value.chars().next() {
        Some('i') => {
            if let Some((n, rest)) =
                encoded_value
                    .split_at(1)
                    .1
                    .split_once('e')
                    .and_then(|(digits, rest)| {
                        let n = digits.parse::<i64>().ok()?;
                        Some((n, rest))
                    })
            {
                return (n.into(), rest);
            }
        }
        Some('l') => {
            let mut values = Vec::new();
            let mut rest = encoded_value.split_at(1).1;
            while !rest.is_empty() && !rest.starts_with('e') {
                let (v, remainder) = decode_bencode_value(rest);
                values.push(v);
                rest = remainder;
            }
            return (values.into(), &rest[1..]);
        }
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut values = Vec::new();
            let mut rest = encoded_value.split_at(1).1;
            while !rest.is_empty() && !rest.starts_with('e') {
                let (k, remainder) = decode_bencode_value(rest);
                let k = match k {
                    serde_json::Value::String(k) => k,
                    k => {
                        panic!("key must be strings, not {k:?}");
                    }
                };
                let (v, remainder) = decode_bencode_value(remainder);
                dict.insert(k, v.clone());
                values.push(v);
                rest = remainder;
            }
            return (dict.into(), &rest[1..]);
        }
        Some('0'..='9') => {
            if let Some((len, rest)) = encoded_value.split_once(':').and_then(|(len, rest)| {
                let len = len.parse::<usize>().ok()?;
                Some((len, rest))
            }) {
                return (rest[..len].to_string().into(), &rest[len..]);
            }
        }
        _ => {}
    }
    panic!("Unhandled encoded value: {}", encoded_value);
}

/// A Metainfo files (also known as .torrent files).
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Torrent {
    /// The URL of the tracker
    announce: String,

    info: Info,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Info {
    /// The suggested name to save the file (or directory) as. It is purely advisory.
    ///
    /// In the single file case, the name key is the name of a file, in the multiple file case,
    /// it's the nmae of a directory.
    name: String,

    /// The number of bytes in each piece the file is split into.
    ///
    /// For the purposes of transfer, files are split into fixed-size pieces which are all the same
    /// length except for possibly the last one which may be truncated. piece length is almost
    /// always a power of two, most commonly 2^18 = 256K
    /// (BitTorrent prior to version 3.2 uses 2^20 = 1 M as default).
    #[serde(rename = "piece length")]
    plength: usize,

    /// Each of which is the SHA1 hash of the piece at the corresponding index.
    pieces: Hashes,

    #[serde(flatten)]
    keys: Keys,
}

/// There is also a key `length` or a key `files`, but not both or neither.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum Keys {
    /// If `length` is present then the download represents a single file,
    SingleFile {
        /// The length of the file in bytes.
        length: usize,
    },

    /// Otherwise it represents a set of files which go in a directory structure.
    /// For the purposes of the other keys in `Info`, the multi-file case is treated as only having
    /// a single file by concatenating the files in the order they appear in the files list.
    MultiFile {
        /// The files list is the value files maps to, and is a list of dictionaries containing the
        /// following keys:
        files: Vec<File>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct File {
    /// The length of the file, in bytes
    length: usize,

    /// Subdirectory names, the last of which is the actual file name
    /// (a zero length list is an error case).
    path: Vec<String>,
}

#[derive(Debug, Clone)]
struct Hashes(Vec<[u8; 20]>);
struct HashesVisitor;

impl<'de> Visitor<'de> for HashesVisitor {
    type Value = Hashes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a byte string whose length is multiple of 20")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.len() % 20 != 0 {
            return Err(E::custom(format!("length is {}", v.len())));
        }

        Ok(Hashes(
            v.chunks_exact(20)
                .map(|slice_20| slice_20.try_into().expect("guaranteed to be length 20"))
                .collect(),
        ))
    }
}

impl<'de> Deserialize<'de> for Hashes {
    fn deserialize<D>(deserializer: D) -> Result<Hashes, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(HashesVisitor)
    }
}

impl Serialize for Hashes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let single_file = self.0.concat();
        serializer.serialize_bytes(&single_file)
    }
}

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
        }
    }

    Ok(())
}
