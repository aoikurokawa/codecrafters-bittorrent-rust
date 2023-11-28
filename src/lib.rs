pub mod download;
pub mod peer;
pub mod torrent;
pub mod tracker;

pub(crate) const BLOCK_MAX: usize = 1 << 14;
