use futures::Future;

use crate::protos::dfs::{Handle, ListReply};
use crate::protos::dfs_grpc::Dfs;
use grpcio::{RpcContext, UnarySink};

#[derive(Clone)]
pub struct DFSServer;

impl Dfs for DFSServer {
    fn list(&mut self, ctx: RpcContext, req: Handle, sink: UnarySink<ListReply>) {
        println!("recv request: {:?}", req);
        let mut rep = ListReply::new();
        rep.set_x("hello".to_owned());

        let f = sink
            .success(rep)
            .map_err(move |e| eprintln!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f);
    }
}
