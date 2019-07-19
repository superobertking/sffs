// #[macro_use]
// extern crate log;

use grpcio::{ChannelBuilder, EnvBuilder};
use sffs::protos::{sffs as ffs, sffs_grpc::SffsClient};

use std::io;
use std::io::prelude::*;
use std::sync::Arc;

fn prompt() {
    print!("$ ");
    io::stdout().flush().expect("Cannot flush stdout.");
}

fn run_cmd(client: &SffsClient, cmd: &str, mut cmd_iter: std::str::SplitWhitespace) -> grpcio::Result<()> {
    match cmd {
        "getdir" => {
            let reply = client.getdir(&ffs::Void::new())?;
            println!("getdir succeeded with {}", reply.get_value());
        }
        "cd" => {
            // cd directory_name
            // TODO
            let path = cmd_iter.next().expect("invalid argument");

            let reply = client.changedir(&path.into())?;
            if reply.get_value() {
                println!("cd succeeded");
            } else {
                println!("cd failed");
            }
        }
        "filecount" => {
            let reply = client.filecount(&ffs::ListOption::new())?;
            println!("filecount succeeded with count of {}", reply.get_value());
        }
        "ls" => {}
        "put" => {}
        "get" => {}
        "randomread" => {}
        c => eprintln!("Unknown command: {:?}", c),
    }
    Ok(())
}

fn main() {
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect("localhost:50051");
    let client = SffsClient::new(ch);

    // for cmdline in io::stdin().lock().lines() {
    loop {
        prompt();

        let mut cmdline = String::new();
        io::stdin().read_line(&mut cmdline).expect("Could not read a line!");

        // EOF
        if cmdline.is_empty() {
            break;
        }

        let mut cmdline_iter = cmdline.split_whitespace();
        let cmd = cmdline_iter.next();
        if cmd.is_none() {
            break;
        }
        let cmd = cmd.unwrap();

        if let Err(e) = run_cmd(&client, &cmd, cmdline_iter) {
            println!("{} failed with {:?}", cmd, e);
        }
    }
}
