use anyhow;
use bytes::{Buf, Bytes};
use std::io::Cursor;

#[derive(Clone, Debug)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Array(Vec<Frame>),
    Null,
}

#[derive(Debug)]
pub enum Error {
    Incomplete,
    Other(anyhow::Error),
}

impl Frame {
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        if !src.has_remaining() {
            return Err(Error::Incomplete);
        }

        match src.chunk()[0] {
            b'+' => get_simple(src),
            b'$' => get_bulk(src),
            b'*' => get_array(src),
            _ => unimplemented!("Not implemented yet"),
        }
    }
}

fn get_simple(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
    src.advance(1);

    let end = get_line(src)?;

    let string =
        String::from_utf8(src.chunk()[..end].to_vec()).map_err(|e| Error::Other(e.into()))?;

    src.advance(end + 2);

    Ok(Frame::Simple(string))
}

fn get_bulk(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
    src.advance(1);

    let end = get_line(src)?;
    let len_str =
        String::from_utf8(src.chunk()[..end].to_vec()).map_err(|e| Error::Other(e.into()))?;

    let len: usize = len_str
        .parse()
        .map_err(|e| Error::Other(anyhow::Error::new(e)))?;

    src.advance(end + 2);

    if src.remaining() < len + 2 {
        return Err(Error::Incomplete);
    }

    let data = Bytes::copy_from_slice(&src.chunk()[..len]);
    src.advance(len + 2);
    Ok(Frame::Bulk(data))
}

fn get_array(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
    src.advance(1);

    let end = get_line(src)?;
    let len_str =
        String::from_utf8(src.chunk()[..end].to_vec()).map_err(|e| Error::Other(e.into()))?;
    let len: usize = len_str
        .parse()
        .map_err(|e| Error::Other(anyhow::Error::new(e)))?;

    src.advance(end + 2);

    let mut out = Vec::with_capacity(len);

    for _ in 0..len {
        out.push(Frame::parse(src)?);
    }

    Ok(Frame::Array(out))
}

fn get_line(src: &mut Cursor<&[u8]>) -> Result<usize, Error> {
    let start = src.position() as usize;
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            return Ok(i - start);
        }
    }

    Err(Error::Incomplete)
}
