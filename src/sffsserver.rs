use futures::Future;
use grpcio::{RpcContext, RpcStatus, RpcStatusCode, UnarySink};
// use nix::unistd;

use crate::protos::{sffs, sffs_grpc::Sffs, MAX_BLOCK_SIZE};

use std::convert::{TryFrom, TryInto};
use std::env;
use std::fs::{self, File, ReadDir};
use std::io::prelude::*;
use std::os::unix::prelude::FileExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
enum NextEntry {
    Dot,    // "."
    DotDot, // ".."
    File,   // any file
}

#[derive(Default)]
struct SFFSServerInner {
    opendir: Mutex<Option<(ReadDir, PathBuf, NextEntry)>>,
    openfile: Mutex<Option<File>>,
}

#[derive(Default, Clone)]
pub struct SFFSServer(Arc<SFFSServerInner>);

impl SFFSServer {
    pub fn new() -> Self {
        Default::default()
    }
}

macro_rules! to_future {
    ($sink:expr, $res:expr) => {
        match $res {
            Some(res) => $sink.success(res),
            None => $sink.fail(RpcStatus::new(RpcStatusCode::Internal, None)),
        }
    };
}

macro_rules! reply {
    ($ctx:expr, $req:expr, $fut:expr) => {
        $ctx.spawn($fut.map_err(move |e| eprintln!("failed to reply {:?}: {:?}", $req, e)));
    };
}

impl SFFSServer {
    fn getdir(&mut self) -> Option<sffs::String> {
        Some(env::current_dir().ok()?.into_os_string().into_string().ok()?.into())
    }
    fn changedir(&mut self, req: &sffs::String) -> Option<sffs::Boolean> {
        Some(env::set_current_dir(req.get_value()).is_ok().into())
    }
    fn filecount(&mut self, req: &sffs::ListOption) -> Option<sffs::Int64> {
        // TODO: filter
        Some((fs::read_dir(".").ok()?.filter(|e| e.is_ok()).count() as i64).into())
    }
    fn openlist(&mut self, req: &sffs::ListRequest) -> Option<sffs::Boolean> {
        let mut guard = self.0.opendir.lock().ok()?;

        let res = if (*guard).is_some() {
            false
        } else {
            *guard = fs::read_dir(req.get_dir())
                .ok()
                .map(|d| (d, req.get_dir().into(), NextEntry::Dot));
            (*guard).is_some()
        };
        drop(guard); // release lock

        Some(res.into())
    }
    fn nextlist(&mut self) -> Option<sffs::DirEntry> {
        // TODO: filter
        let mut guard = self.0.opendir.lock().ok()?;

        let (ref mut dir, ref path, ref mut next) = guard.as_mut()?;

        let mut getnext = || -> Option<sffs::DirEntry> {
            // There will definitely be one entry, even may be empty.
            Some(loop {
                match *next {
                    // next is still file
                    NextEntry::File => match dir.next() {
                        Some(entry) => break sffs::DirEntry::try_from(entry.ok()?).ok()?,
                        None => break sffs::DirEntry::default(),
                    },
                    NextEntry::Dot => {
                        *next = NextEntry::DotDot;
                        let meta = File::open(path.join(".")).ok()?.metadata().ok()?;
                        break (".".to_owned(), meta).try_into().ok()?;
                    }
                    NextEntry::DotDot => {
                        *next = NextEntry::File;
                        let meta = File::open(path.join("..")).ok()?.metadata().ok()?;
                        break ("..".to_owned(), meta).try_into().ok()?;
                    }
                }
            })
        };
        loop {
            let entry = getnext();
            if entry.is_some() {
                return entry;
            }
        }
    }
    fn closelist(&mut self) -> Option<sffs::Boolean> {
        let mut guard = self.0.opendir.lock().ok()?;
        Some((*guard).take().is_some().into())
    }
    fn openfiletoread(&mut self, req: &sffs::String) -> Option<sffs::Boolean> {
        let mut guard = self.0.openfile.lock().ok()?;

        let res = if (*guard).is_some() {
            false
        } else {
            *guard = File::open(req.get_value()).ok();
            (*guard).is_some()
        };
        Some(res.into())
    }
    fn openfiletowrite(&mut self, req: &sffs::String) -> Option<sffs::Boolean> {
        let mut guard = self.0.openfile.lock().ok()?;

        let res = if (*guard).is_some() {
            false
        } else {
            *guard = File::create(req.get_value()).ok();
            (*guard).is_some()
        };
        Some(res.into())
    }
    fn nextread(&mut self) -> Option<sffs::Block> {
        let mut guard = self.0.openfile.lock().ok()?;

        let ref mut file = guard.as_mut()?;

        let mut buf = vec![0u8; MAX_BLOCK_SIZE];
        let len = file.read(&mut buf).unwrap_or(0);
        buf.truncate(len);
        Some(buf.into())
    }
    fn nextwrite(&mut self, req: &sffs::Block) -> Option<sffs::Boolean> {
        let mut guard = self.0.openfile.lock().ok()?;

        let ref mut file = guard.as_mut()?;

        let res = file.write(req.get_data()).is_ok();
        Some(res.into())
    }
    fn randomread(&mut self, req: &sffs::Range) -> Option<sffs::Block> {
        let mut guard = self.0.openfile.lock().ok()?;;

        let ref mut file = guard.as_mut()?;

        let mut buf = vec![0u8; req.get_count() as usize];
        let len = file.read_at(&mut buf, req.get_start() as u64).unwrap_or(0);
        buf.truncate(len);
        Some(buf.into())
    }
    fn closefile(&mut self) -> Option<sffs::Boolean> {
        let mut guard = self.0.openfile.lock().ok()?;;
        Some((*guard).take().is_some().into())
    }
}

impl Sffs for SFFSServer {
    #[inline]
    fn getdir(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::String>) {
        reply!(ctx, req, to_future!(sink, self.getdir()));
    }
    #[inline]
    fn changedir(&mut self, ctx: RpcContext, req: sffs::String, sink: UnarySink<sffs::Boolean>) {
        reply!(ctx, req, to_future!(sink, self.changedir(&req)));
    }
    #[inline]
    fn filecount(&mut self, ctx: RpcContext, req: sffs::ListOption, sink: UnarySink<sffs::Int64>) {
        reply!(ctx, req, to_future!(sink, self.filecount(&req)));
    }
    #[inline]
    fn openlist(&mut self, ctx: RpcContext, req: sffs::ListRequest, sink: UnarySink<sffs::Boolean>) {
        reply!(ctx, req, to_future!(sink, self.openlist(&req)));
    }
    #[inline]
    fn nextlist(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::DirEntry>) {
        reply!(ctx, req, to_future!(sink, self.nextlist()));
    }
    #[inline]
    fn closelist(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::Boolean>) {
        reply!(ctx, req, to_future!(sink, self.closelist()));
    }
    #[inline]
    fn openfiletoread(&mut self, ctx: RpcContext, req: sffs::String, sink: UnarySink<sffs::Boolean>) {
        reply!(ctx, req, to_future!(sink, self.openfiletoread(&req)));
    }
    #[inline]
    fn openfiletowrite(&mut self, ctx: RpcContext, req: sffs::String, sink: UnarySink<sffs::Boolean>) {
        reply!(ctx, req, to_future!(sink, self.openfiletowrite(&req)));
    }
    #[inline]
    fn nextread(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::Block>) {
        reply!(ctx, req, to_future!(sink, self.nextread()));
    }
    #[inline]
    fn nextwrite(&mut self, ctx: RpcContext, req: sffs::Block, sink: UnarySink<sffs::Boolean>) {
        reply!(ctx, req, to_future!(sink, self.nextwrite(&req)));
    }
    #[inline]
    fn randomread(&mut self, ctx: RpcContext, req: sffs::Range, sink: UnarySink<sffs::Block>) {
        reply!(ctx, req, to_future!(sink, self.randomread(&req)));
    }
    #[inline]
    fn closefile(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::Boolean>) {
        reply!(ctx, req, to_future!(sink, self.closefile()));
    }
}
