// #[macro_use]
// extern crate log;

use futures::sync::oneshot;
use futures::Future;
use grpcio::{EnvBuilder, ServerBuilder};
use sffs::protos::sffs_grpc::create_sffs;
use sffs::sffsserver::SFFSServer;
use sffs::common;

use std::io::{self, Read};
use std::sync::Arc;
use std::thread;

fn main() {
    let host = std::env::args().skip(1).next().unwrap_or("127.0.0.1".to_owned());
    let env = Arc::new(EnvBuilder::new().build());
    let service = create_sffs(SFFSServer::new());
    // let service = helloworld::create_greeter(GreeterService);
    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind(host, common::COMM_PORT)
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
