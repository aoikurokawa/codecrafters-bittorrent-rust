use std::env;

use serde::Deserialize;
use serde_bencode;
use serde_json;

/// A Metainfo file (also known as .torrent files).
#[derive(Debug, Clone, Deserialize)]
struct Torrent {
    /// The URL of the tracker.
    announce: reqwest::Url,

    info: Info,
}

#[derive(Debug, Clone, Deserialize)]
struct Info {
    /// The suggested name to save the file (or directory) as. It is purely advisory.
    name: String,

    /// The number of bytes in each piece the file is split into.
    ///
    ///
    #[serde(rename = "piece length")]
    plength: usize,

    /// pieces maps to a string whose length is a multiple of 20. It is to be subdivided into
    /// strings of length 20.
    pieces: Vec<u8>,

    keys: Keys,
}

#[derive(Debug, Clone, Deserialize)]
struct Keys {}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.0.to_string());
    } else {
        eprintln!("unknown command: {}", args[1])
    }
}
