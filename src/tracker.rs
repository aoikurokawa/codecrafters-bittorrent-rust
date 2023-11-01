use serde::{Deserialize, Serialize, Serializer};

use crate::peers::Peers;

#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
    /// The info hash of torrent.
    #[serde(serialize_with = "urlencode")]
    pub info_hash: [u8; 20],

    /// A unique identifier for your client.
    ///
    /// A string of length 20 that you get to pick.
    pub peer_id: String,

    /// The port your client is listening on.
    pub port: u16,

    /// The total amount uploaded so far.
    pub uploaded: usize,

    /// The total amount downloaded so far.
    pub downloaded: usize,

    /// The number of bytes left to download.
    pub left: usize,

    /// WHether the peer lsit should use the [compact representation](https://www.bittorrent.org/beps/bep_0023.html)
    ///
    /// The compact representation is more commonly used in the wild, the non-compact
    /// representation is mostly supported for back-ward compatibility.
    ///
    pub compact: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackerResponse {
    /// An integer, indicating how often your client should make a request to the tracker in
    /// seconds.
    pub interval: usize,

    /// A string, which contains list of peers that your client can connect to.
    pub peers: Peers,
}

fn urlencode<S>(t: &[u8; 20], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    serializer.serialize_str(&encoded)
}
