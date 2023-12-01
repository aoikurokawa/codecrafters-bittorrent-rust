use std::collections::BinaryHeap;

use anyhow::Context;
use futures_util::StreamExt;
use sha1::{Digest, Sha1};

use crate::{
    peer::Peer,
    piece::Piece,
    torrent::{File, Keys, Torrent},
    tracker::TrackerResponse,
    BLOCK_MAX,
};

pub async fn download_all(t: &Torrent) -> anyhow::Result<Downloaded> {
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
        .buffer_unordered(5);
    while let Some((peer_addr, peer)) = peers.next().await {
        match peer {
            Ok(peer) => {
                peer_list.push(peer);
                if peer_list.len() >= 5 {
                    break;
                }
            }
            Err(e) => {
                eprintln!("failed to connect to peer {peer_addr:?}: {e}");
            }
        }
    }
    drop(peers);

    let mut peers = peer_list;
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

    let mut all_pieces = vec![0; t.length()];
    while let Some(piece) = need_pieces.pop() {
        let piece_size = piece.length();

        // the + (BLOCK_MAX - 1) rounds up
        let nblocks = (piece_size + (BLOCK_MAX - 1)) / BLOCK_MAX;
        let peers = peers
            .iter_mut()
            .enumerate()
            .filter_map(|(peer_i, peer)| piece.peers().contains(&peer_i).then_some(peer))
            .collect::<Vec<&mut Peer>>();

        let (submit, tasks) = kanal::bounded_async(nblocks);
        for block in 0..nblocks {
            submit
                .send(block)
                .await
                .expect("bound holds all these items");
        }

        let (finish, mut done) = tokio::sync::mpsc::channel(nblocks);
        let mut participants = futures_util::stream::futures_unordered::FuturesUnordered::new();
        for peer in peers {
            participants.push(peer.participate(
                piece.index(),
                piece_size,
                nblocks,
                submit.clone(),
                tasks.clone(),
                finish.clone(),
            ));
        }
        drop(submit);
        drop(finish);
        drop(tasks);

        let mut all_blocks = vec![0u8; piece_size];
        let mut bytes_received = 0;
        loop {
            tokio::select! {
                joined = participants.next(), if !participants.is_empty() => {
                    // if a participant ends early, it's either slow or failed
                    match joined {
                        None => {
                            // there are no peers
                            // this must mean we are about to get None from done.recv()
                            // so we'll handle it there
                        }
                        Some(Ok(_)) => {
                            // the peer gave up because it timed out
                            // nothing to do, except maybe de-prioritize this peer for later
                            // TODO:
                        }
                        Some(Err(_)) => {
                            // the peer failed and should be removed
                            // it already isn't participating in this piece any more, so this is
                            // more of an indicator that we shouldn't try this peer again, and
                            // should remove it from the global peer list
                            // TODO:
                        }
                    }

                }
                piece = done.recv() => {
                    if let Some(piece) = piece {
                    // keep track of the bytes in message
                        let piece = crate::peer::Piece::ref_from_bytes(&piece.payload[..])
                            .expect("always get all Piece response fields from peer");
                        all_blocks[piece.begin() as usize..][..piece.block().len()].copy_from_slice(piece.block());
                        bytes_received += piece.block().len();
                        if bytes_received == piece_size {
                            // have received every piece
                            // this must mean that all participation have either exited or are waiting
                            // for more work -- in either case, it is okay to drop all the participant
                            // futures.
                        }
                    } else {
                        // there are no peers left, so we can't progress!
                        break;
                    }
                }
            }
        }
        drop(participants);

        if bytes_received == piece_size {
            // great, we got all the bytes
        } else {
            // we'll need to connect to more peers, and make sure that those additional peers also
            // have this piece, and then download the piece we _didn't_ get from them.
            // probably also stick this back onto the pices_heap
            anyhow::bail!("no peers left to get piece {}", piece.index());
        }

        let mut hasher = Sha1::new();
        hasher.update(&all_blocks);
        let hash: [u8; 20] = hasher.finalize().try_into().expect("");
        assert_eq!(hash, piece.hash());

        all_pieces[piece.index() * t.info.plength..][..piece_size].copy_from_slice(&all_blocks);
    }

    Ok(Downloaded {
        bytes: all_pieces,
        files: match &t.info.keys {
            Keys::SingleFile { length } => vec![File {
                length: *length,
                path: vec![t.info.name.clone()],
            }],
            Keys::MultiFile { files } => files.clone(),
        },
    })
}

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
