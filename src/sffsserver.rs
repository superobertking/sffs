use futures::Future;
use grpcio::{RpcContext, RpcStatus, RpcStatusCode, UnarySink};
// use nix::unistd;

use crate::protos::sffs;
use crate::protos::sffs_grpc::Sffs;

use std::convert::TryFrom;
use std::env;
use std::fs::{self, DirEntry, File, ReadDir};
use std::io;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct SFFSServerInner {
    opendir: Mutex<Option<(ReadDir,)>>,
    openfile: Mutex<Option<File>>,
}

#[derive(Default, Clone)]
pub struct SFFSServer(Arc<SFFSServerInner>);

impl SFFSServer {
    pub fn new() -> Self {
        Default::default()
    }
}

macro_rules! reply {
    ($ctx:expr, $req:expr, $fut:expr) => {
        $ctx.spawn($fut.map_err(move |e| eprintln!("failed to reply {:?}: {:?}", $req, e)));
    };
}

impl Sffs for SFFSServer {
    fn getdir(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::String>) {
        let mut res: Option<String> = None;
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(cwd) = cwd.into_os_string().into_string() {
                res = Some(cwd);
            }
        }
        let f = match res {
            Some(cwd) => sink.success(cwd.into()),
            None => sink.fail(RpcStatus::new(RpcStatusCode::Internal, None)),
        };
        reply!(ctx, req, f);
    }

    fn changedir(&mut self, ctx: RpcContext, req: sffs::String, sink: UnarySink<sffs::Boolean>) {
        let res = env::set_current_dir(req.get_value()).is_ok();
        reply!(ctx, req, sink.success(res.into()));
    }

    fn filecount(&mut self, ctx: RpcContext, req: sffs::ListOption, sink: UnarySink<sffs::Int64>) {
        let f = match fs::read_dir(".") {
            Ok(dir) => sink.success((dir.filter(|e| e.is_ok()).count() as i64).into()),
            Err(_) => sink.fail(RpcStatus::new(RpcStatusCode::Internal, None)),
        };
        reply!(ctx, req, f);
    }

    fn openlist(&mut self, ctx: RpcContext, req: sffs::ListRequest, sink: UnarySink<sffs::Boolean>) {
        let mut guard = match self.0.opendir.lock() {
            Ok(guard) => guard,
            Err(_) => {
                reply!(ctx, req, sink.fail(RpcStatus::new(RpcStatusCode::Internal, None)));
                return;
            }
        };

        let res = if (*guard).is_some() {
            false
        } else {
            match fs::read_dir(req.get_dir()) {
                Ok(dir) => {
                    *guard = Some((dir,));
                    true
                }
                Err(_) => false,
            }
        };
        drop(guard); // release lock

        reply!(ctx, req, sink.success(res.into()));
    }

    fn nextlist(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::DirEntry>) {
        // TODO: return . and ..

        let mut guard = match self.0.opendir.lock() {
            Ok(guard) => guard,
            Err(_) => {
                reply!(ctx, req, sink.fail(RpcStatus::new(RpcStatusCode::Internal, None)));
                return;
            }
        };

        let f = match *guard {
            Some((ref mut dir,)) => loop {
                match dir.next() {
                    Some(entry) => {
                        if let Ok(e) = entry {
                            if let Ok(e) = sffs::DirEntry::try_from(e) {
                                break sink.success(e);
                            }
                        }
                    }
                    None => break sink.success(Default::default()),
                }
            },
            None => sink.fail(RpcStatus::new(RpcStatusCode::InvalidArgument, None)),
        };
        drop(guard); // release lock

        reply!(ctx, req, f);
    }
    fn closelist(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::Boolean>) {
        let mut guard = match self.0.opendir.lock() {
            Ok(guard) => guard,
            Err(_) => {
                reply!(ctx, req, sink.fail(RpcStatus::new(RpcStatusCode::Internal, None)));
                return;
            }
        };

        let res = (*guard).is_some();
        *guard = None;
        drop(guard); // release lock

        reply!(ctx, req, sink.success(res.into()));
    }
    fn openfiletoread(&mut self, ctx: RpcContext, req: sffs::String, sink: UnarySink<sffs::Boolean>) {
        unimplemented!();
    }
    fn openfiletowrite(&mut self, ctx: RpcContext, req: sffs::String, sink: UnarySink<sffs::Boolean>) {
        unimplemented!();
    }
    fn nextread(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::Block>) {
        unimplemented!();
    }
    fn nextwrite(&mut self, ctx: RpcContext, req: sffs::Block, sink: UnarySink<sffs::Boolean>) {
        unimplemented!();
    }
    fn randomread(&mut self, ctx: RpcContext, req: sffs::Range, sink: UnarySink<sffs::Block>) {
        unimplemented!();
    }
    fn closefile(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::Boolean>) {
        unimplemented!();
    }
}
