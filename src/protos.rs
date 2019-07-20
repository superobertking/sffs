#![allow(bare_trait_objects)]

pub mod sffs;
pub mod sffs_grpc;

pub const MAX_BLOCK_SIZE: usize = 512;

impl From<bool> for sffs::Boolean {
    #[inline]
    fn from(b: bool) -> Self {
        Self {
            value: b,
            ..Default::default()
        }
    }
}

impl From<i64> for sffs::Int64 {
    #[inline]
    fn from(x: i64) -> Self {
        Self {
            value: x,
            ..Default::default()
        }
    }
}

impl From<&str> for sffs::String {
    #[inline]
    fn from(s: &str) -> Self {
        Self {
            value: s.to_owned(),
            ..Default::default()
        }
    }
}

impl From<String> for sffs::String {
    #[inline]
    fn from(s: String) -> Self {
        Self {
            value: s,
            ..Default::default()
        }
    }
}

use std::convert::TryFrom;
use std::fs;
use std::io;
use std::time::SystemTime;

impl TryFrom<fs::DirEntry> for sffs::DirEntry {
    type Error = io::Error;
    #[inline]
    fn try_from(e: fs::DirEntry) -> Result<Self, Self::Error> {
        let meta = e.metadata()?;
        let mtime = meta
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "SystemTime before UNIX EPOCH!"))?;
        Ok(Self {
            name: e.file_name().into_string().unwrap(),
            isdir: meta.is_dir(),
            size: meta.len() as i64,
            modifytime: mtime.as_secs() as i64,
            ..Default::default()
        })
    }
}

impl From<Vec<u8>> for sffs::Block {
    #[inline]
    fn from(b: Vec<u8>) -> Self {
        // panic if length exceeded
        assert!(
            b.len() <= MAX_BLOCK_SIZE,
            "buf length={} longer than MAX_BLOCK_SIZE={}",
            b.len(),
            MAX_BLOCK_SIZE
        );

        Self {
            data: b,
            ..Default::default()
        }
    }
}

use std::ops::Range;

impl From<(i64, i64)> for sffs::Range {
    #[inline]
    fn from(r: (i64, i64)) -> Self {
        Self {
            start: r.0,
            count: r.1,
            ..Default::default()
        }
    }
}
