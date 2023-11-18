use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{peers::Peers, torrent::Torrent};

/// Note: the info hash field is _not_ included.
#[derive(Debug, Clone, Serialize)]
pub struct TrackerRequest {
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

impl TrackerResponse {
    pub(crate) async fn query(t: &Torrent) -> anyhow::Result<Self> {
        let info_hash = t.info_hash();
        let request = TrackerRequest {
            peer_id: String::from("00112233445566778899"),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: t.length(),
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

        Ok(tracker_info)
    }
}

pub fn urlencode(t: &[u8; 20]) -> String {
    let mut encoded = String::with_capacity(3 * t.len());
    for &byte in t {
        encoded.push('%');
        encoded.push_str(&hex::encode(&[byte]));
    }
    encoded
}
