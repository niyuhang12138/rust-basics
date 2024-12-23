use crate::Value;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum KvError {
    #[error("Not found for table: {0}, key: {1}")]
    NotFound(String, String),

    #[error("Cannot parse command: `{0}`")]
    InvalidCommand(String),

    #[error("Cannot covert value {:?} to {}", ._0, ._1)]
    ConvertError(Value, &'static str),

    #[error("Cannot process command {0} with table: {1}, key: {2}, Error: {3}")]
    StorageError(&'static str, String, String, String),

    #[error("Failed to encode protobuf message")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Failed to decode protobuf message")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Failed to access sled db")]
    SledError(#[from] sled::Error),

    #[error("Failed to access rocks db")]
    RocksError(#[from] rocksdb::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}
