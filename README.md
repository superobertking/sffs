# SFFS

Simple Flat File Service/System

## Compilation

### Dependencies

- Rust compiler
  - 1.36.0 or higher versions are verified
- protobuf compiler (protoc)
- Go compiler
  - install Go compiler or manually convert proto file to rs bindings. ([guide](https://github.com/pingcap/grpc-rs))

### Building

Entering the following command to launch the generate the targtets.

```shell
cargo build
```

There would be three executables in `/target/debug` directory. 

`single` is the individual program which is required in part 1. `client` and `server` are used for achieve functions of filesystem over RPC communication.

## Group Members

- [superobertking](https://github.com/superobertking)
  - **Contribution**
    - Design communication details of the whole PRC process
    - Implement the required access operations on both client and server sides
- [DapperX](https://github.com/DapperX)
  - **Contribution**
    - Write the single program to test directory operations locally
    - Run unit and functional tests and help debugging

## Enclosed files

- client.rs    *client program, mainly implementing input parsing, sending request and printing results*
- common.rs    *store constants and structures that are shared among the whole project*
- error.rs    *define all kinds all error types that are used during the runtime*
- filter.rs    *support the trait for filtering at part 5*
- lib.rs    *rust default library file for export used modules*
- op_dir.rs *single program for verifying directory operations*
- protos.rs    *interface module utilities*
- server.rs    *server binary program, simply providing entrance*
- sffsserver.rs    *library for server*
- protos/
  - sffs_grpc.rs    define gRPC interfaces
  - sffs.proto    *gRPC prototypes*
  - sffs.rs    *helper functions for application invocations*

