// #[macro_use]
// extern crate log;

use futures::sync::oneshot;
use futures::Future;
use grpcio::{EnvBuilder, ServerBuilder};
use rpctest::dfsserver::DFSServer;
use rpctest::protos::dfs_grpc::create_dfs;

use std::io::{self, Read};
use std::sync::Arc;
use std::thread;

fn main() {
    let env = Arc::new(EnvBuilder::new().build());
    let service = create_dfs(DFSServer);
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
