use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Tracker {
    /// The info hash of the torrent
    info_hash: [u8; 20],

    /// A unique identifier for your client
    ///
    /// A string of length 20 that you get to pick. 
    peer_id: String,

    /// The port your client is listening on
    port: u16,

    /// The total amount uploaded so far
    uploaded: usize,

    /// The total amount downloaded so far
    downloaded: usize,

    /// The number of bytes left to download
    left: usize,

    /// whether the peer list should use the compact representation
    /// 
    /// The compact representation is more commonly used in the wild, the non-compact representation is mostly supported for backward-compatibility.
    compact: u8,
}
