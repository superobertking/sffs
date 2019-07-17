// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_DFS_LIST: ::grpcio::Method<super::dfs::Handle, super::dfs::ListReply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/DFS/list",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct DfsClient {
    client: ::grpcio::Client,
}

impl DfsClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        DfsClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn list_opt(&self, req: &super::dfs::Handle, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::dfs::ListReply> {
        self.client.unary_call(&METHOD_DFS_LIST, req, opt)
    }

    pub fn list(&self, req: &super::dfs::Handle) -> ::grpcio::Result<super::dfs::ListReply> {
        self.list_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_async_opt(&self, req: &super::dfs::Handle, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::dfs::ListReply>> {
        self.client.unary_call_async(&METHOD_DFS_LIST, req, opt)
    }

    pub fn list_async(&self, req: &super::dfs::Handle) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::dfs::ListReply>> {
        self.list_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Dfs {
    fn list(&mut self, ctx: ::grpcio::RpcContext, req: super::dfs::Handle, sink: ::grpcio::UnarySink<super::dfs::ListReply>);
}

pub fn create_dfs<S: Dfs + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DFS_LIST, move |ctx, req, resp| {
        instance.list(ctx, req, resp)
    });
    builder.build()
}
