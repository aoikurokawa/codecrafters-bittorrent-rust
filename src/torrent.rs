use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::hashes::Hashes;

/// A Metainfo file (also known as .torrent files).
#[derive(Debug, Clone, Deserialize)]
pub struct Torrent {
    /// The URL of the tracker.
    pub announce: String,

    /// This maps to a dictionary, with keys described below.
    pub info: Info,
}

impl Torrent {
    pub fn info_hash(&self) -> [u8; 20] {
        let info_encoded = serde_bencode::to_bytes(&self.info).expect("re-encode info session");
        let mut hasher = Sha1::new();
        hasher.update(&info_encoded);
        hasher
            .finalize()
            .try_into()
            .expect("GenericArray<_, 20> == [_; 20]")
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Info {
    /// The suggested name to save the file (or directory) as. It is purely advisory.
    pub name: String,

    /// The number of bytes in each piece the file is split into.
    ///
    /// For the purpose of transfer, files are split into fixed-size pieces which are all the same
    /// length excet for possibly the last one which may be truncated. piece length is almost
    /// always a power of two, most commonly 2^18 = 256K (BitTorrent prior to version 3.2 uses 2^20
    /// = 1M as default).
    #[serde(rename = "piece length")]
    pub plength: usize,

    /// Each entry of `pieces` is the SHA1 hash of the piece at the corresponding index.
    pub pieces: Hashes,

    #[serde(flatten)]
    pub keys: Keys,
}

/// There is a key `length` or a key `files`, but not both or neither.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Keys {
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
        files: Vec<File>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct File {
    /// The length of the file, in bytes
    pub length: usize,

    /// Subdirectory names for this file, the last of which is the actual file name
    /// (a zero length list is an error case).
    pub path: Vec<String>,
}