use std::{collections::BinaryHeap, net::SocketAddrV4};

use anyhow::Context;
use futures_util::StreamExt;

use crate::{
    peer::Peer,
    piece::Piece,
    torrent::{File, Torrent},
    tracker::TrackerResponse,
};

pub(crate) async fn all(t: &Torrent) -> anyhow::Result<Downloaded> {
    let info_hash = t.info_hash();
    let peer_info = TrackerResponse::query(t, info_hash)
        .await
        .context("query tracker for peer info")?;

    let mut peer_list = Vec::new();
    let mut peers = futures_util::stream::iter(peer_info.peers.0.iter())
        .map(|&peer_addr| async move {
            let peer = Peer::new(peer_addr, info_hash).await;
            (peer_addr, peer)
        })
        .buffer_unordered(3);
    while let Some((peer_addr, peer)) = peers.next().await {
        match peer {
            Ok(peer) => {
                peer_list.push(peer);
                if peer_list.len() >= 5 {
                    break;
                }
            }
            Err(e) => eprintln!("failed to connect to peer {peer_addr:?}: {e:?}"),
        }
    }

    drop(peers);
    let peers = peer_list;

    let mut need_pieces = BinaryHeap::new();
    let mut no_peers = Vec::new();
    for piece_i in 0..t.info.pieces.0.len() {
        let piece = Piece::new(piece_i, &t, &peers);
        if piece.peers().is_empty() {
            no_peers.push(piece);
        } else {
            need_pieces.push(piece);
        }
    }

    assert!(no_peers.is_empty());

    while let Some(piece) = need_pieces.pop() {

    }

    Ok(Downloaded {
        bytes: todo!(),
        files: todo!(),
    })
}

pub async fn download_piece(
    candidate_peers: &[SocketAddrV4],
    piece_hash: [u8; 20],
    piece_size: usize,
) {
}

pub async fn download_piece_block_from(peer: &SocketAddrV4, block_i: usize, block_size: usize) {}

pub struct Downloaded {
    bytes: Vec<u8>, // TODO: maybe Bytes,
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
    downloaded: &'d Downloaded,
    file_iter: std::slice::Iter<'d, File>,
    offset: usize,
}

impl<'d> DownloadedIter<'d> {
    fn new(d: &'d Downloaded) -> Self {
        Self {
            downloaded: d,
            file_iter: d.files.iter(),
            offset: 0,
        }
    }
}

impl<'d> Iterator for DownloadedIter<'d> {
    type Item = DownloadedFile<'d>;

    fn next(&mut self) -> Option<Self::Item> {
        let file = self.file_iter.next()?;
        let byte = &self.downloaded.bytes[self.offset..][..file.length];
        Some(DownloadedFile { file, byte })
    }
}

pub struct DownloadedFile<'d> {
    file: &'d File,
    byte: &'d [u8],
}

impl<'d> DownloadedFile<'d> {
    pub fn path(&self) -> &'d [String] {
        &self.file.path
    }

    pub fn bytes(&self) -> &'d [u8] {
        self.byte
    }
}
