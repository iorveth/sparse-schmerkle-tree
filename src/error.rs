use crate::H256;

#[derive(Debug, Clone)]
pub enum Error {
    MissingKey(H256),
}

pub type Result<T> = ::std::result::Result<T, Error>;
