// #[macro_use]
// extern crate log;
use chrono::prelude::*;

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
        "getdir" | "pwd" => {
            let reply = client.getdir(&ffs::Void::new())?;
            println!("getdir succeeded with {}", reply.get_value());
        }
        "cd" => {
            // cd directory_name
            let path = match cmd_iter.next() {
                Some(path) => path,
                None => {
                    eprintln!("cd: invalid argument");
                    return Ok(());
                }
            };

            let reply = client.changedir(&path.into())?;
            if reply.get_value() {
                println!("cd succeeded");
            } else {
                println!("cd failed");
            }
        }
        "filecount" => {
            // TODO: filter
            let reply = client.filecount(&ffs::ListOption::new())?;
            println!("filecount succeeded with count of {}", reply.get_value());
        }
        "ls" => {
            // TODO: filter
            let mut cmd_iter = cmd_iter.peekable();
            let longopt = if let Some(&"-l") = cmd_iter.peek() {
                cmd_iter.next();
                true
            } else {
                false
            };
            let path = cmd_iter.next().unwrap_or(".");

            // open list
            let mut request = ffs::ListRequest::new();
            request.set_dir(path.to_owned());
            let reply = client.openlist(&request)?;
            if !reply.get_value() {
                eprintln!("ls failed");
                return Ok(());
            }

            // do list
            loop {
                let entry = client.nextlist(&ffs::Void::new())?;
                if entry.get_name().is_empty() {
                    break;
                }

                let tail = ["", "/"][entry.get_isdir() as usize];
                if longopt {
                    let time = Utc
                        .timestamp(entry.get_modifytime(), 0)
                        .format("%a\t%b\t%d\t%T\t%Z\t%Y");
                    println!("{}{}\t{}\t{}", entry.get_name(), tail, entry.get_size(), time);
                } else {
                    println!("{}{}", entry.get_name(), tail);
                }
            }

            // close list
            let reply = client.closelist(&ffs::Void::new())?;
            if !reply.get_value() {
                eprintln!("ls failed");
            }
        }
        "put" => {
            // TODO: check if same dir
        }
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
            println!("{} failed with rpc error: {:?}", cmd, e);
        }
    }
}
