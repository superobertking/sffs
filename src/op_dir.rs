use std::io::prelude::*;
use std::io;
use std::time::SystemTime;
use std::env;
use std::fs::{self, File, ReadDir, DirEntry};
use std::path::PathBuf;
use chrono::prelude::*;

enum EntryType {
    Current,
    Parent,
    Regualr,
}

enum ExDirEntry {
    Current,
    Parent,
    Entry(DirEntry),
}

fn proto_getdir() -> Option<String> {
    env::current_dir().ok()?.into_os_string().into_string().ok()
}

fn proto_changedir(dir: &str) -> bool {
    env::set_current_dir(dir).is_ok()
}

fn proto_filecount() -> Option<u64> {
    Some(fs::read_dir(".").ok()?.count() as u64 + 2)
}

fn proto_openlist(base: &str) -> Option<(ReadDir, &str, EntryType)> {
    Some((fs::read_dir(base).ok()?, base, EntryType::Current))
}

fn proto_nextlist(ld: &mut (ReadDir, &str, EntryType)) -> Option<io::Result<ExDirEntry>> {
    loop {
        match &ld.2 {
            &EntryType::Current => {
                ld.2 = EntryType::Parent;
                break Some(Ok(ExDirEntry::Current));
            }
            &EntryType::Parent => {
                ld.2 = EntryType::Regualr;
                break Some(Ok(ExDirEntry::Parent));
            }
            &EntryType::Regualr => {
                break ld.0.next().map(|r| r.map(|d| ExDirEntry::Entry(d)));
            }
        }
    }
}

fn proto_closelist(_: (ReadDir, &str, EntryType)) {
}

fn main() {
    println!("[info] Started for testing directory operations locally.");
    loop {
        print!("$ ");
        io::stdout().flush();

        let mut cmdline = String::new();
        io::stdin().read_line(&mut cmdline).expect("Could not read a line!");

        if cmdline.is_empty() { break; }

        let mut args = cmdline.split_whitespace();
        let program = match args.next() {
            Some(first) => first,
            None => continue,
        };

        println!("[debug] parsed program name: {}", program);

        match program {
            "getdir" => {
                match proto_getdir() {
                    Some(cwd) => println!("getdir succeeded with {}", cwd),
                    None => eprintln!("Failed to query current directory"),
                }
            }

            "cd" => {
                let path = match args.next() {
                    Some(path) => path,
                    None => {
                        println!("Failed to change directory");
                        continue
                    },
                };

                let is_success = proto_changedir(path);
                if is_success { println!("cd succeeded"); }
                else { println!("cd failed"); }
            }

            "filecount" => {
                let res = proto_filecount();
                match res {
                    Some(cnt) => println!("filecount succeeded with count of {}", cnt),
                    None => println!("failed to count files"),
                }
            }

            "ls" => {
                let mut args = args.peekable();
                let longopt = if let Some(&"-l") = args.peek() {
                    args.next();
                    true
                }
                else { false };

                let path = args.next().unwrap_or(".");
                let mut ld = match proto_openlist(path) {
                    Some(ld) => ld,
                    None => {
                        println!("Failed to open directorry");
                        continue;
                    },
                };
                loop {
                    let dentry = proto_nextlist(&mut ld);
                    if dentry.is_none() { break; }
                    let (name, meta) = match dentry.unwrap() {
                        Ok(exentry) => {
                            match exentry {
                                ExDirEntry::Current => {
                                    let path = PathBuf::from(ld.1).join(".");
                                    if let Ok(e) = File::open(path) {
                                        if let Ok(meta) = e.metadata() {
                                            (".".to_owned(), meta)
                                        } else {
                                            continue;
                                        }
                                    } else {
                                        continue;
                                    }
                                },
                                ExDirEntry::Parent => {
                                    let path = PathBuf::from(ld.1).join("..");
                                    if let Ok(e) = File::open(path) {
                                        if let Ok(meta) = e.metadata() {
                                            ("..".to_owned(), meta)
                                        } else {
                                            continue;
                                        }
                                    } else {
                                        continue;
                                    }
                                },
                                ExDirEntry::Entry(e) => {
                                    if let Ok(meta) = e.metadata() {
                                        (e.file_name().into_string().unwrap(), meta)
                                    } else {
                                        continue;
                                    }
                                },
                            }
                        },
                        Err(_) => continue,
                    };
                    print!("{}", name);
                    if meta.is_dir() { print!("/"); }
                    if longopt {
                        let time = Utc
                            .timestamp(meta.modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64, 0)
                            .format("%a\t%b\t%d\t%T\t%Z\t%Y");
                        print!("\t{}\t{}", meta.len(), time);
                    }
                    print!("\n");
                }
                proto_closelist(ld);
            }

            unknown => eprintln!("Unknown command: {:?}", unknown),
        }
    }
}