use std::error::Error;
use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, ExecuteError>;

#[derive(Debug)]
pub enum ExecuteError {
    IO(io::Error),
    RPC(grpcio::Error),
    Common(CommonErrorKind),
    Custom(Box<dyn Error + Send + Sync>),
}

#[derive(Debug, Clone, Copy)]
pub enum CommonErrorKind {
    Generic,
    InvalidArgument,
    NotFound,
    CloseFail,
}

impl fmt::Display for ExecuteError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "hello")
    }
}

impl Error for ExecuteError {}

impl From<io::Error> for ExecuteError {
    #[inline]
    fn from(e: io::Error) -> Self {
        ExecuteError::IO(e)
    }
}

impl From<grpcio::Error> for ExecuteError {
    #[inline]
    fn from(e: grpcio::Error) -> Self {
        ExecuteError::RPC(e)
    }
}

impl From<CommonErrorKind> for ExecuteError {
    #[inline]
    fn from(e: CommonErrorKind) -> Self {
        ExecuteError::Common(e)
    }
}

impl From<&'static str> for ExecuteError {
    #[inline]
    fn from(s: &'static str) -> Self {
        ExecuteError::Custom(s.into())
    }
}

impl From<String> for ExecuteError {
    #[inline]
    fn from(s: String) -> Self {
        ExecuteError::Custom(s.into())
    }
}
