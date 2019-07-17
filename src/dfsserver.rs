use futures::Future;
use grpcio::{RpcContext, UnarySink};
use nix::unistd;

use crate::protos::dfs;
use crate::protos::dfs_grpc::Dfs;

#[derive(Clone)]
pub struct DFSServer;

impl Dfs for DFSServer {
    fn getdir(&mut self, ctx: RpcContext, req: dfs::Void, sink: UnarySink<dfs::String>) {
        let cwd = unistd::getcwd()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap();

        let mut reply = dfs::String::new();
        reply.set_value(cwd);

        let f = sink
            .success(reply)
            .map_err(move |e| eprintln!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f);
    }

    fn changedir(&mut self, ctx: RpcContext, req: dfs::String, sink: UnarySink<dfs::Boolean>) {
        let res = unistd::chdir(req.get_value()).is_ok();

        let mut reply = dfs::Boolean::new();
        reply.set_value(res);

        let f = sink
            .success(reply)
            .map_err(move |e| eprintln!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f);
    }
    fn filecount(&mut self, ctx: RpcContext, req: dfs::ListOption, sink: UnarySink<dfs::Int64>) {
        // TODO
        let count = std::fs::read_dir(".").unwrap().count();

        let mut reply = dfs::Int64::new();
        reply.set_value(count as i64);

        let f = sink
            .success(reply)
            .map_err(move |e| eprintln!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f);
    }
    fn openlist(&mut self, ctx: RpcContext, req: dfs::ListRequest, sink: UnarySink<dfs::Boolean>) {
        unimplemented!();
    }
    fn nextlist(&mut self, ctx: RpcContext, req: dfs::Void, sink: UnarySink<dfs::DirEntry>) {
        unimplemented!();
    }
    fn closelist(&mut self, ctx: RpcContext, req: dfs::Void, sink: UnarySink<dfs::Boolean>) {
        unimplemented!();
    }
    fn openfiletoread(&mut self, ctx: RpcContext, req: dfs::String, sink: UnarySink<dfs::Boolean>) {
        unimplemented!();
    }
    fn openfiletowrite(
        &mut self,
        ctx: RpcContext,
        req: dfs::String,
        sink: UnarySink<dfs::Boolean>,
    ) {
        unimplemented!();
    }
    fn nextread(&mut self, ctx: RpcContext, req: dfs::Void, sink: UnarySink<dfs::FileData>) {
        unimplemented!();
    }
    fn nextwrite(&mut self, ctx: RpcContext, req: dfs::FileData, sink: UnarySink<dfs::Boolean>) {
        unimplemented!();
    }
    fn randomread(&mut self, ctx: RpcContext, req: dfs::Range, sink: UnarySink<dfs::FileData>) {
        unimplemented!();
    }
    fn closefile(&mut self, ctx: RpcContext, req: dfs::Void, sink: UnarySink<dfs::Boolean>) {
        unimplemented!();
    }

    /* fn list(&mut self, ctx: RpcContext, req: Handle, sink: UnarySink<ListReply>) {
        println!("recv request: {:?}", req);
        let mut rep = ListReply::new();
        rep.set_x("hello".to_owned());

        let f = sink
            .success(rep)
            .map_err(move |e| eprintln!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f);
    } */
}
