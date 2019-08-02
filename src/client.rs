// #[macro_use]
// extern crate log;
// #![feature(let_chains)]
use chrono::prelude::*;
use grpcio::{ChannelBuilder, EnvBuilder};
use nix::unistd;

use sffs::filter::MetaDataFilter;
use sffs::protos::{sffs as ffs, sffs_grpc::SffsClient, MAX_BLOCK_SIZE};
use sffs::CommonErrorKind::{InvalidArgument, NotFound};
use sffs::common;

use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::sync::Arc;

fn prompt() {
    print!("$ ");
    io::stdout().flush().expect("Cannot flush stdout.");
}

struct RemoteFile<'a> {
    client: Option<&'a SffsClient>,
    isdir: bool,
}

impl<'a> RemoteFile<'a> {
    fn open(client: &'a SffsClient, name: &str) -> sffs::Result<Self> {
        let is_found = client.openfiletoread(&name.into())?;
        Self::_new(client, name, false, is_found.get_value())
    }

    fn openlist(client: &'a SffsClient, name: &str, option: Option<String>) -> sffs::Result<Self> {
        let mut request = ffs::ListRequest::new();
        request.set_dir(name.to_owned());
        if let Some(option) = option {
            request.set_option(option.into());
        }

        let is_found = client.openlist(&request)?;
        Self::_new(client, name, true, is_found.get_value())
    }

    fn create(client: &'a SffsClient, name: &str) -> sffs::Result<Self> {
        let is_found = client.openfiletowrite(&name.into())?;
        Self::_new(client, name, false, is_found.get_value())
    }

    #[inline]
    fn _new(client: &'a SffsClient, name: &str, isdir: bool, is_found: bool) -> sffs::Result<Self> {
        if is_found {
            Ok(Self {
                client: Some(client),
                isdir,
            })
        } else {
            Err(NotFound(name.to_owned()).into())
        }
    }

    fn close(&mut self) -> sffs::Result<()> {
        let client = match self.client.take() {
            Some(client) => client,
            None => return Ok(()),
        };
        let reply = if self.isdir {
            client.closelist(&ffs::Void::new())?
        } else {
            client.closefile(&ffs::Void::new())?
        };
        if reply.get_value() {
            Ok(())
        } else {
            Err(sffs::CommonErrorKind::CloseFail.into())
        }
    }
}

impl<'a> Drop for RemoteFile<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            eprintln!("{}", e);
        }
    }
}

fn run_cmd(client: &SffsClient, cmd: &str, mut cmd_iter: std::str::SplitWhitespace) -> sffs::Result<()> {
    match cmd {
        "getdir" | "pwd" => {
            let is_found = client.getdir(&ffs::Void::new())?;
            println!("getdir succeeded with {}", is_found.get_value());
        }
        "cd" => {
            // cd directory_name
            let path = cmd_iter.next().ok_or(InvalidArgument)?;

            let is_success = client.changedir(&path.into())?;
            if is_success.get_value() {
                println!("cd succeeded");
            } else {
                println!("cd failed");
            }
        }
        "filecount" => {
            let mut option = ffs::ListOption::new();
            if let Some(o) = cmd_iter.next() {
                option.set_option(o.to_owned());
            }
            let reply = client.filecount(&option)?;
            println!("filecount succeeded with count of {}", reply.get_value());
        }
        "ls" => {
            let mut cmd_iter = cmd_iter.peekable();
            let longopt = if let Some(&"-l") = cmd_iter.peek() {
                cmd_iter.next();
                true
            } else {
                false
            };
            let (path, option) = match cmd_iter.next() {
                Some(token) => {
                    if MetaDataFilter::is_valid_pattern(token) {
                        (cmd_iter.next().unwrap_or("."), Some(token)) // token as option
                    } else {
                        (token, cmd_iter.next()) // token as path
                    }
                }
                None => (".", None), // there won't be another token afterwards
            };
            let option = option.map(|s| s.to_owned());

            // open list
            let mut remotelist = RemoteFile::openlist(client, path, option)?;

            // traverse list
            loop {
                let entry = client.nextlist(&ffs::Void::new())?;
                if entry.get_name().is_empty() {
                    break;
                }

                print!("{}", entry.get_name());
                if entry.get_isdir() { print!("/"); }
                if longopt {
                    let time = Utc
                        .timestamp(entry.get_modifytime(), 0)
                        .format("%a\t%b\t%d\t%T\t%Z\t%Y");
                    print!("\t{}\t{}", entry.get_size(), time);
                }
                print!("\n");
            }

            // close list
            remotelist.close()?;
        }
        "get" => {
            let remotepath = cmd_iter.next().ok_or(InvalidArgument)?;
            let localpath = cmd_iter.next().unwrap_or(remotepath);

            // open remote file
            let mut remotefile = RemoteFile::open(client, remotepath)?;

            // open local file
            let _ = unistd::unlink(localpath);
            let mut localfile = File::create(localpath).map_err(|_| "cannot open local file as write")?;

            let mut bytes = 0usize;
            // read remote data to local file
            loop {
                let reply = client.nextread(&ffs::Void::new())?;
                if reply.get_data().is_empty() {
                    break;
                }
                localfile.write(reply.get_data())?;
                bytes += reply.get_data().len();
            }

            // close remote file
            remotefile.close()?;
            // local file closed after drop
            drop(localfile);

            println!("get succeeded transferring {} bytes", bytes);
        }
        "put" => {
            let localpath = cmd_iter.next().ok_or(InvalidArgument)?;
            let remotepath = cmd_iter.next().unwrap_or(localpath);

            let mut localfile = File::open(localpath).map_err(|_| NotFound(localpath.to_owned()))?;

            // open remote file
            let mut remotefile = RemoteFile::create(client, remotepath)?;

            let mut bytes = 0usize;
            // read local file to remote file
            loop {
                let mut buf = vec![0u8; MAX_BLOCK_SIZE];

                let len = localfile.read(&mut buf).unwrap_or(0);
                if len == 0 {
                    break;
                }
                buf.truncate(len);

                if client.nextwrite(&buf.into())?.get_value() {
                    bytes += len;
                } else {
                    eprintln!("put failed: cannot write remote file");
                }
            }

            // close remote file
            remotefile.close()?;
            // local file closed after drop
            drop(localfile);

            println!("put succeeded transferring {} bytes", bytes);
        }
        "randomread" => {
            use InvalidArgument as InvArg;
            let remotepath = cmd_iter.next().ok_or(InvArg)?;
            let range_start = cmd_iter.next().ok_or(InvArg)?.parse::<i64>().map_err(|_| InvArg)?;
            let range_count = cmd_iter.next().ok_or(InvArg)?.parse::<i64>().map_err(|_| InvArg)?;

            if !(0 <= range_count && range_count as usize <= MAX_BLOCK_SIZE) {
                return Err(InvArg.into());
            }

            // open remote file
            let mut remotefile = RemoteFile::open(client, remotepath)?;
            // read remote data to stdout
            let reply = client.randomread(&(range_start, range_count).into())?;
            // close remote file
            remotefile.close()?;

            println!("randomread succeeded transferring {} bytes", reply.get_data().len());
            print!("{}", String::from_utf8_lossy(&reply.get_data()));
        }
        c => eprintln!("Unknown command: {:?}", c),
    }
    Ok(())
}

fn usage(prog_name: &str) -> ! {
    println!("Usage: {} <hostname> [-f <script>]", prog_name);
    ::std::process::exit(1);
}

// -> std::process::ExitCode
fn main() {
    let mut args = std::env::args();
    let prog_name = args.next().expect("Cannot get program name");

    let mut addr = args.next().unwrap_or_else(|| usage(&prog_name));
    addr.push_str(":");
    addr.push_str(&common::COMM_PORT.to_string());

    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect(&addr);
    let client = SffsClient::new(ch);

    let mut inputstream: Box<dyn BufRead>;
    let isfile: bool;
    if let Some(o) = args.next() {
        if o == "-f" {
            if let Some(filename) = args.next() {
                let file = File::open(filename).expect(&format!("Cannot open script file {}", prog_name));
                inputstream = Box::new(BufReader::new(file));
                isfile = true;
            } else {
                usage(&prog_name);
            }
        } else {
            usage(&prog_name);
        }
    } else {
        inputstream = Box::new(BufReader::new(io::stdin()));
        isfile = false;
    };

    // for cmdline in io::stdin().lock().lines() {
    loop {
        if !isfile {
            prompt();
        }

        let mut cmdline = String::new();
        inputstream.read_line(&mut cmdline).expect("Could not read a line!");

        // EOF (with no '\n')
        if cmdline.is_empty() {
            break;
        }

        let mut cmdline_iter = cmdline.split_whitespace();
        let cmd = match cmdline_iter.next() {
            Some(cmd) => cmd,
            None => continue, //continue when command line is empty
        };

        if let Err(e) = run_cmd(&client, &cmd, cmdline_iter) {
            use sffs::ExecuteError::{Common, Custom, IO, RPC};
            match e {
                IO(e) => eprintln!("{} failed with I/O Error {}", cmd, e),
                RPC(e) => eprintln!("{} failed with RPC Error {}", cmd, e),
                Common(e) => eprintln!("{} failed {}", cmd, e),
                Custom(e) => eprintln!("{} failed {}", cmd, e),
            }
        }
    }
}
