use std::net::SocketAddrV4;

use anyhow::Context;
use bytes::{Buf, BufMut, BytesMut};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::{Decoder, Encoder, Framed};

const BLOCK_MAX: usize = 1 << 14;

pub(crate) struct Peer {
    addr: SocketAddrV4,
    stream: Framed<TcpStream, MessageFramer>,
    bitfield: Bitfield,
}

impl Peer {
    pub async fn new(peer_addr: SocketAddrV4, info_hash: [u8; 20]) -> anyhow::Result<Self> {
        let mut peer = tokio::net::TcpStream::connect(peer_addr)
            .await
            .context("connect to peer")?;
        let mut handshake = Handshake::new(info_hash, *b"00112233445566778899");
        {
            let handshake_bytes = handshake.as_bytes_mut();

            peer.write_all(handshake_bytes)
                .await
                .context("write handshake")?;

            peer.read_exact(handshake_bytes)
                .await
                .context("read handshake")?;
        }

        anyhow::ensure!(handshake.length == 19);
        anyhow::ensure!(&handshake.bittorent == b"BitTorrent protocol");

        let mut peer = tokio_util::codec::Framed::new(peer, MessageFramer);
        let bitfield = peer
            .next()
            .await
            .expect("peer always send a bitfield")
            .context("peer message was invalid")?;
        anyhow::ensure!(bitfield.tag == MessageTag::Bitfield);

        Ok(Self {
            addr: peer_addr,
            stream: peer,
            bitfield: Bitfield::from_payload(bitfield.payload),
        })
    }

    pub async fn download(
        &mut self,
        piece_i: u32,
        block_i: u32,
        block_size: u32,
    ) -> anyhow::Result<Vec<u8>> {
        let mut request = Request::new(
            piece_i as u32,
            block_i * BLOCK_MAX as u32,
            block_size as u32,
        );
        let request_bytes = Vec::from(request.as_bytes_mut());
        self.stream
            .send(Message {
                tag: MessageTag::Request,
                payload: request_bytes,
            })
            .await
            .with_context(|| format!("send request for block {block_i}"))?;

        let piece = &self
            .stream
            .next()
            .await
            .expect("peer always send a piece")
            .context("peer message was invalid")?;

        anyhow::ensure!(piece.tag == MessageTag::Piece);
        anyhow::ensure!(!piece.payload.is_empty());

        let piece = Piece::ref_from_bytes(&piece.payload[..])
            .expect("always get all Piece response fields from peer");

        anyhow::ensure!(piece.index() == piece_i);
        anyhow::ensure!(piece.begin() == block_i * BLOCK_MAX as u32);
        anyhow::ensure!(piece.block().len() as u32 == block_size);

        Ok(Vec::from(piece.block()))
    }
}

pub struct Bitfield {
    payload: Vec<u8>,
}

impl Bitfield {
    pub(crate) fn has_piece(&self, piece_i: usize) -> bool {
        let byte_i = piece_i / (u8::BITS as usize);
        let bit_i = (piece_i % (u8::BITS as usize)) as u32;
        let Some(&byte) = self.payload.get(byte_i) else {
            return false;
        };

        // 01010101
        //
        // nth bit is set
        //
        // 1 << n
        //
        // 01010101
        // &
        // 00001000
        // 00000000
        //
        // 10000000
        byte & 1_u8.rotate_right(bit_i + 1) != 0
    }

    pub(crate) fn pieces(&self) -> impl Iterator<Item = usize> + '_ {
        self.payload.iter().enumerate().flat_map(|(byte_i, byte)| {
            (0..u8::BITS).filter_map(move |bit_i| {
                let piece_i = byte_i * (u8::BITS as usize) + (bit_i as usize);
                let mask = 1u8.rotate_right(bit_i + 1); // == 128

                (byte & mask != 0).then_some(piece_i)
            })
        })
    }

    fn from_payload(payload: Vec<u8>) -> Self {
        Self { payload }
    }
}

#[repr(C)]
pub struct Handshake {
    pub length: u8,
    pub bittorent: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(info_hash: [u8; 20], peer_id: [u8; 20]) -> Self {
        Self {
            length: 19,
            bittorent: *b"BitTorrent protocol",
            reserved: [0; 8],
            info_hash,
            peer_id,
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let handshake_bytes = self as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];

        // Safety: Handshaek is a POD with repr(C)
        let handshake_bytes: &mut [u8; std::mem::size_of::<Handshake>()] =
            unsafe { &mut *handshake_bytes };

        handshake_bytes
    }
}

#[repr(C)]
pub struct Request {
    index: [u8; 4],
    begin: [u8; 4],
    length: [u8; 4],
}

impl Request {
    pub fn new(index: u32, begin: u32, length: u32) -> Self {
        Self {
            index: index.to_be_bytes(),
            begin: begin.to_be_bytes(),
            length: length.to_be_bytes(),
        }
    }

    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }

    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }

    pub fn length(&self) -> u32 {
        u32::from_be_bytes(self.length)
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let bytes = self as *mut Self as *mut [u8; std::mem::size_of::<Self>()];

        // Safety: Handshaek is a POD with repr(C)
        let bytes: &mut [u8; std::mem::size_of::<Self>()] = unsafe { &mut *bytes };

        bytes
    }
}

#[repr(C)]
pub struct Piece<T: ?Sized = [u8]> {
    index: [u8; 4],
    begin: [u8; 4],
    block: T,
}

impl Piece {
    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }

    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }

    pub fn block(&self) -> &[u8] {
        &self.block
    }

    const PIECE_LEAD: usize = std::mem::size_of::<Piece<()>>();
    pub fn ref_from_bytes(data: &[u8]) -> Option<&Self> {
        if data.len() < Self::PIECE_LEAD {
            return None;
        }

        let n = data.len();

        let piece = &data[..n - Self::PIECE_LEAD] as *const [u8] as *const Piece;

        Some(unsafe { &*piece })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageTag {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub tag: MessageTag,
    pub payload: Vec<u8>,
}

pub struct MessageFramer;

const MAX: usize = 1 << 16;

impl Decoder for MessageFramer {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // Not enough data to read length marker + tag.
            return Ok(None);
        }

        // Read length marker.
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        if length == 0 {
            // this is a heartbeat message.
            // discard it.
            src.advance(4);
            // and then try again in case the buffer has more messages
            return self.decode(src);
        }

        if src.len() < 5 {
            // Not enough data to read tag marger.
            return Ok(None);
        }

        // Check that the length is not too large to avoid a denial of
        // service attack where the server runs out of memory.
        if length > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", length),
            ));
        }

        if src.len() < 4 + length {
            // The full string has not yet arrived.
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(4 + length - src.len());

            // We inform the Framed that we need more bytes to form the next
            // frame.
            return Ok(None);
        }

        // Use advance to modify src such that it no longer contains
        // this frame.
        let tag = match src[4] {
            0 => MessageTag::Choke,
            1 => MessageTag::Unchoke,
            2 => MessageTag::Interested,
            3 => MessageTag::NotInterested,
            4 => MessageTag::Have,
            5 => MessageTag::Bitfield,
            6 => MessageTag::Request,
            7 => MessageTag::Piece,
            8 => MessageTag::Cancel,
            tag => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown message type {}", tag),
                ))
            }
        };
        let data = if src.len() > 5 {
            src[5..4 + length].to_vec()
        } else {
            Vec::new()
        };
        src.advance(4 + length);

        Ok(Some(Message {
            tag: tag as MessageTag,
            payload: data,
        }))
    }
}

// struct MessageFramer {}

impl Encoder<Message> for MessageFramer {
    type Error = std::io::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Don't send a message if it is longer than the other end will
        // accept.
        if item.payload.len() + 1 > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", item.payload.len()),
            ));
        }

        // Convert the length into a byte array.
        // The cast to u32 cannot overflow due to the length check above.
        let len_slice = u32::to_be_bytes(item.payload.len() as u32 + 1);

        // Reserve space in the buffer.
        dst.reserve(4 + 1 + item.payload.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&len_slice);
        dst.put_u8(item.tag as u8);
        dst.extend_from_slice(&item.payload);
        Ok(())
    }
}
