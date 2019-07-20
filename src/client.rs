// #[macro_use]
// extern crate log;
use chrono::prelude::*;

use grpcio::{ChannelBuilder, EnvBuilder};
use sffs::protos::{sffs as ffs, sffs_grpc::SffsClient, MAX_BLOCK_SIZE};

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::sync::Arc;

fn prompt() {
    print!("$ ");
    io::stdout().flush().expect("Cannot flush stdout.");
}

fn run_cmd(client: &SffsClient, cmd: &str, mut cmd_iter: std::str::SplitWhitespace) -> grpcio::Result<()> {
    macro_rules! closeremotefile {
        ($client:expr, $cmd:expr) => {
            // close remote file
            let reply = $client.closefile(&ffs::Void::new())?;
            if !reply.get_value() {
                eprintln!("{} failed: cannot close remote file", $cmd);
            }
        };
    };

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
        "get" => {
            let remotepath = match cmd_iter.next() {
                Some(path) => path,
                None => {
                    eprintln!("get failed: invalid argument");
                    return Ok(());
                }
            };
            let localpath = cmd_iter.next().unwrap_or(remotepath);

            let mut localfile = match File::create(localpath) {
                Ok(file) => file,
                Err(_) => {
                    eprintln!("get failed: cannot open local file");
                    return Ok(());
                }
            };

            // open remote file
            let reply = client.openfiletoread(&remotepath.to_owned().into())?;
            if !reply.get_value() {
                eprintln!("get failed {} not found", remotepath);
                return Ok(());
            }

            let mut bytes = 0usize;
            // read remote data to local file
            loop {
                let reply = match client.nextread(&ffs::Void::new()) {
                    Ok(reply) => reply,
                    Err(e) => {
                        closeremotefile!(client, cmd);
                        return Err(e);
                    }
                };

                if reply.get_data().is_empty() {
                    break;
                }

                if let Err(e) = localfile.write(reply.get_data()) {
                    eprintln!("get failed: local file write error {:?}", e);
                    closeremotefile!(client, cmd);
                    return Ok(());
                }

                bytes += reply.get_data().len();
            }

            // close remote file
            closeremotefile!(client, cmd);

            // local file closed after drop
            drop(localfile);

            println!("get succeeded transferring {} bytes", bytes);
        }
        "put" => {
            // TODO: check if same dir
            let localpath = match cmd_iter.next() {
                Some(path) => path,
                None => {
                    eprintln!("put failed: invalid argument");
                    return Ok(());
                }
            };
            let remotepath = cmd_iter.next().unwrap_or(localpath);

            let mut localfile = match File::open(localpath) {
                Ok(file) => file,
                Err(_) => {
                    eprintln!("put failed: cannot open local file");
                    return Ok(());
                }
            };

            // open remote file
            let reply = client.openfiletowrite(&remotepath.to_owned().into())?;
            if !reply.get_value() {
                eprintln!("put failed");
                return Ok(());
            }

            let mut bytes = 0usize;
            // read local file to remote file
            loop {
                let mut buf = vec![0u8; MAX_BLOCK_SIZE];

                let len = localfile.read(&mut buf).unwrap_or(0);
                if len == 0 {
                    break;
                }
                buf.truncate(len);
                bytes += len;

                match client.nextwrite(&buf.into()) {
                    Ok(reply) => {
                        if !reply.get_value() {
                            eprintln!("put failed: cannot write remote file");
                            closeremotefile!(client, cmd);
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        closeremotefile!(client, cmd);
                        return Err(e);
                    }
                };
            }

            // close remote file
            closeremotefile!(client, cmd);

            // local file closed after drop
            drop(localfile);

            println!("put succeeded transferring {} bytes", bytes);
        }
        "randomread" => {
            // TODO: fix error handling
            let remotepath = match cmd_iter.next() {
                Some(path) => path,
                None => {
                    eprintln!("randomread failed: invalid argument");
                    return Ok(());
                }
            };
            let range_start = match cmd_iter.next() {
                Some(x) => match x.parse::<i64>() {
                    Ok(x) => x,
                    Err(_) => {
                        eprintln!("randomread failed: invalid argument");
                        return Ok(());
                    }
                },
                None => {
                    eprintln!("randomread failed: invalid argument");
                    return Ok(());
                }
            };
            let range_count = match cmd_iter.next() {
                Some(x) => match x.parse::<i64>() {
                    Ok(x) if 0 <= x && x as usize <= MAX_BLOCK_SIZE => x,
                    _ => {
                        eprintln!("randomread failed: invalid argument");
                        return Ok(());
                    }
                },
                None => {
                    eprintln!("randomread failed: invalid argument");
                    return Ok(());
                }
            };

            // open remote file
            let reply = client.openfiletoread(&remotepath.to_owned().into())?;
            if !reply.get_value() {
                eprintln!("randomread failed {} not found", remotepath);
                return Ok(());
            }

            // read remote data to stdout
            let reply = match client.randomread(&(range_start, range_count).into()) {
                Ok(reply) => reply,
                Err(e) => {
                    closeremotefile!(client, cmd);
                    return Err(e);
                }
            };

            // close remote file
            closeremotefile!(client, cmd);

            println!("randomread succeeded transferring {} bytes", reply.get_data().len());

            print!("{}", String::from_utf8_lossy(&reply.get_data()));
        }
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
            continue;
        }
        let cmd = cmd.unwrap();

        if let Err(e) = run_cmd(&client, &cmd, cmdline_iter) {
            println!("{} failed with rpc error: {:?}", cmd, e);
        }
    }
}
