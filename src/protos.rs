#![allow(bare_trait_objects)]

pub mod sffs;
pub mod sffs_grpc;

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
