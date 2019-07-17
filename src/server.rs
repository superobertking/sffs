// #[macro_use]
// extern crate log;

use futures::sync::oneshot;
use futures::Future;
use grpcio::{EnvBuilder, RpcContext, ServerBuilder, UnarySink};
use protos::{
    dfs::{Handle, ListReply},
    dfs_grpc::{create_dfs, Dfs},
};

use std::io::{self, Read};
use std::sync::Arc;
use std::thread;

#[derive(Clone)]
struct DFSService;

impl Dfs for DFSService {
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

fn main() {
    let env = Arc::new(EnvBuilder::new().build());
    let service = create_dfs(DFSService);
    // let service = helloworld::create_greeter(GreeterService);
    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind("127.0.0.1", 50_051)
        .build()
        .unwrap();
    server.start();
    for &(ref host, port) in server.bind_addrs() {
        println!("listening on {}:{}", host, port);
    }

    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        println!("Press ENTER to exit...");
        let _ = io::stdin().read(&mut [0]).unwrap();
        tx.send(())
    });
    let _ = rx.wait();
    let _ = server.shutdown().wait();
}
