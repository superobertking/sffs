// #[macro_use]
// extern crate log;

use grpcio::{ChannelBuilder, EnvBuilder};
use rpctest::protos::{dfs::Handle, dfs_grpc::DfsClient};

use std::sync::Arc;

fn main() {
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect("localhost:50051");
    let client = DfsClient::new(ch);

    let mut req = Handle::new();
    req.set_x(1);
    let reply = client.list(&req).expect("rpc failure");
    println!("List received: {}", reply.get_x());
}
