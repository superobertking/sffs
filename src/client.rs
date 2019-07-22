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

struct RemoteFile<'a> {
    client: Option<&'a SffsClient>,
    isdir: bool,
}

impl<'a> RemoteFile<'a> {
    fn open(client: &'a SffsClient, name: String) -> sffs::Result<Self> {
        let reply = client.openfiletoread(&name.into())?;
        Self::_new(client, false, reply.get_value())
    }

    fn openlist(client: &'a SffsClient, name: String, option: Option<String>) -> sffs::Result<Self> {
        let mut request = ffs::ListRequest::new();
        request.set_dir(name);
        if let Some(option) = option {
            request.set_option(option.into());
        }

        let reply = client.openlist(&request)?;
        Self::_new(client, false, reply.get_value())
    }

    fn create(client: &'a SffsClient, name: String) -> sffs::Result<Self> {
        let reply = client.openfiletowrite(&name.into())?;
        Self::_new(client, false, reply.get_value())
    }

    #[inline]
    fn _new(client: &'a SffsClient, isdir: bool, reply: bool) -> sffs::Result<Self> {
        if reply {
            Ok(Self {
                client: Some(client),
                isdir,
            })
        } else {
            Err(sffs::CommonErrorKind::NotFound.into())
        }
    }

    fn close(&mut self) -> sffs::Result<()> {
        let client = self.client.take();
        if client.is_none() {
            return Ok(());
        }
        let client = client.unwrap();
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
            let path = cmd_iter.next().ok_or("cd: invalid argument")?;

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
            let mut remotelist = RemoteFile::openlist(client, path.to_owned(), None)?;

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
            let remotepath = cmd_iter.next().ok_or("get failed: invalid argument")?;
            let localpath = cmd_iter.next().unwrap_or(remotepath);

            let mut localfile = File::create(localpath).map_err(|_| "cannot open local file")?;

            // open remote file
            let mut remotefile = RemoteFile::open(client, remotepath.to_owned())?;

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
            let localpath = cmd_iter.next().ok_or("put failed: invalid argument")?;
            let remotepath = cmd_iter.next().unwrap_or(localpath);

            let mut localfile = File::open(localpath).map_err(|_| "put failed: cannot open local file")?;

            // open remote file
            let mut remotefile = RemoteFile::create(client, remotepath.to_owned())?;

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
            // TODO: fix error handling
            let invarg = sffs::CommonErrorKind::InvalidArgument;

            let remotepath = cmd_iter.next().ok_or(invarg)?;

            let range_start = cmd_iter.next().ok_or(invarg)?.parse::<i64>().map_err(|_| invarg)?;
            let range_count = cmd_iter.next().ok_or(invarg)?.parse::<i64>().map_err(|_| invarg)?;

            if 0 <= range_count && range_count as usize <= MAX_BLOCK_SIZE {
                return Err(invarg.into());
            }

            // open remote file
            let mut remotefile = RemoteFile::open(client, remotepath.to_owned())?;

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
            use sffs::ExecuteError::{Common, Custom, IO, RPC};
            match e {
                IO(e) => eprintln!("{} failed with I/O Error {}", cmd, e),
                RPC(e) => eprintln!("{} failed with RPC Error {}", cmd, e),
                Common(e) => eprintln!("{:?}", e),
                Custom(e) => eprintln!("{}", e),
            }
        }
    }
}
