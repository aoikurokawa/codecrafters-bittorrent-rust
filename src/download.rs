use std::net::SocketAddrV4;

use anyhow::{Context, Ok};

use crate::{
    torrent::{File, Torrent},
    tracker::TrackerResponse,
};

pub(crate) async fn download_all(t: &Torrent) -> anyhow::Result<Downloaded> {
    let peer_info = TrackerResponse::query(t)
        .await
        .context("query tracker for peer info")?;

    Ok(Downloaded {
        bytes: todo!(),
        files: todo!(),
    })
}

pub async fn download_piece(
    candiate_peers: &[SocketAddrV4],
    piece_hash: [u8; 20],
    piece_size: usize,
) {
}

pub async fn download_piece_block_from(peer: &SocketAddrV4, block_i: usize, block_size: usize) {}

pub struct Downloaded {
    bytes: Vec<u8>,
    files: Vec<File>,
}

impl<'a> IntoIterator for &'a Downloaded {
    type Item = DownloadedFile<'a>;
    type IntoIter = DownloadedIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DownloadedIter::new(self)
    }
}

pub struct DownloadedIter<'d> {
    downloaed: &'d Downloaded,
    file_iter: std::slice::Iter<'d, File>,
    offset: usize,
}

impl<'d> DownloadedIter<'d> {
    pub fn new(d: &'d Downloaded) -> Self {
        Self {
            downloaed: d,
            file_iter: d.files.iter(),
            offset: 0,
        }
    }
}

impl<'d> Iterator for DownloadedIter<'d> {
    type Item = DownloadedFile<'d>;

    fn next(&mut self) -> Option<Self::Item> {
        let file = self.file_iter.next()?;
        let bytes = &self.downloaed.bytes[self.offset..][..file.length];

        Some(DownloadedFile { file, bytes })
    }
}

pub struct DownloadedFile<'d> {
    pub file: &'d File,
    pub bytes: &'d [u8],
}

impl<'d> DownloadedFile<'d> {
    pub fn path(&self) -> &'d [String] {
        &self.file.path
    }

    pub fn bytes(&self) -> &'d [u8] {
        self.bytes
    }
}
