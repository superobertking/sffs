// #[macro_use]
// extern crate log;
// #![feature(let_chains)]
use chrono::prelude::*;
use grpcio::{ChannelBuilder, EnvBuilder};

use sffs::filter::MetaDataFilter;
use sffs::protos::{sffs as ffs, sffs_grpc::SffsClient, MAX_BLOCK_SIZE};
use sffs::CommonErrorKind::{InvalidArgument, NotFound};

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
        let reply = client.openfiletoread(&name.into())?;
        Self::_new(client, name, false, reply.get_value())
    }

    fn openlist(client: &'a SffsClient, name: &str, option: Option<String>) -> sffs::Result<Self> {
        let mut request = ffs::ListRequest::new();
        request.set_dir(name.to_owned());
        if let Some(option) = option {
            request.set_option(option.into());
        }

        let reply = client.openlist(&request)?;
        Self::_new(client, name, true, reply.get_value())
    }

    fn create(client: &'a SffsClient, name: &str) -> sffs::Result<Self> {
        let reply = client.openfiletowrite(&name.into())?;
        Self::_new(client, name, false, reply.get_value())
    }

    #[inline]
    fn _new(client: &'a SffsClient, name: &str, isdir: bool, reply: bool) -> sffs::Result<Self> {
        if reply {
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
            let reply = client.getdir(&ffs::Void::new())?;
            println!("getdir succeeded with {}", reply.get_value());
        }
        "cd" => {
            // cd directory_name
            let path = cmd_iter.next().ok_or(InvalidArgument)?;

            let reply = client.changedir(&path.into())?;
            if reply.get_value() {
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
            remotelist.close()?;
        }
        "get" => {
            let remotepath = cmd_iter.next().ok_or(InvalidArgument)?;
            let localpath = cmd_iter.next().unwrap_or(remotepath);

            // open local file
            let mut localfile = File::create(localpath).map_err(|_| "cannot open local file as write")?;
            // open remote file
            let mut remotefile = RemoteFile::open(client, remotepath)?;

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
            // TODO: check if same dir
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
                bytes += len;

                if client.nextwrite(&buf.into())?.get_value() {
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

            if 0 <= range_count && range_count as usize <= MAX_BLOCK_SIZE {
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
    panic!("Usage: {} [-f <script>]", prog_name);
}

// -> std::process::ExitCode
fn main() {
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect("localhost:50051");
    let client = SffsClient::new(ch);

    let mut args = std::env::args();
    let prog_name = args.next().expect("Cannot get program name");
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

        // EOF
        if cmdline.is_empty() {
            break;
        }

        let mut cmdline_iter = cmdline.split_whitespace();
        let cmd = cmdline_iter.next();
        // continue when input line is empty
        if cmd.is_none() {
            continue;
        }
        let cmd = cmd.unwrap();

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
