use futures::Future;
use grpcio::{RpcContext, RpcStatus, RpcStatusCode, UnarySink};
// use nix::unistd;

use crate::protos::sffs;
use crate::protos::sffs_grpc::Sffs;

use std::fs::{DirEntry, File, ReadDir};
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
        let fut = fut.map_err(move |e| eprintln!("failed to reply {:?}: {:?}", $req, e));
        $ctx.spawn(f);
    };
}

impl Sffs for SFFSServer {
    fn getdir(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::String>) {
        let res: Option<String> = None;
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(cwd) = cwd.into_os_string().into_string() {
                res = Some(cwd);
            }
        }
        let f = match res {
            Some(cwd) => sink.success(cwd),
            None => sink.fail(RpcStatus::new(RpcStatusCode::Internal, None)),
        };
        reply!(ctx, req, f);
    }

    fn changedir(&mut self, ctx: RpcContext, req: sffs::String, sink: UnarySink<sffs::Boolean>) {
        let res = std::env::set_current_dir(req.get_value()).is_ok();
        reply!(ctx, req, sink.success(res.into()));
    }

    fn filecount(&mut self, ctx: RpcContext, req: sffs::ListOption, sink: UnarySink<sffs::Int64>) {
        let count = std::fs::read_dir(".").filter(|e| e.is_ok()).count() as i64;
        reply!(ctx, req, sink.success(count.into()));
    }

    fn openlist(&mut self, ctx: RpcContext, req: sffs::ListRequest, sink: UnarySink<sffs::Boolean>) {
        unimplemented!();
    }
    fn nextlist(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::DirEntry>) {
        unimplemented!();
    }
    fn closelist(&mut self, ctx: RpcContext, req: sffs::Void, sink: UnarySink<sffs::Boolean>) {
        unimplemented!();
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
