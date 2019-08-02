pub mod error;
pub mod filter;
pub mod protos;
pub mod sffsserver;
pub mod common;

pub use error::{CommonErrorKind, ExecuteError, Result};
pub use sffsserver::SFFSServer;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
