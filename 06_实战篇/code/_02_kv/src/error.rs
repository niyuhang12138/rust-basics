use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Frame is larger than max size")]
    FrameError,

    #[error("Cannot parse command: `{0}`")]
    InvalidCommand(String),

    #[error("Cannot covert value {:?} to {}", ._0, ._1)]
    ConvertError(String, &'static str),

    #[error("Cannot process command {0} with table: {1}, key: {2}, Error: {3}")]
    StorageError(&'static str, String, String, String),

    #[error("Certificate parser error: to load {0} {1}")]
    CertifcateParseError(&'static str, &'static str),

    #[error("Failed to encode protobuf message")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Failed to decode protobuf message")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Failed to access sled db")]
    SledError(#[from] sled::Error),

    #[error("I/O error")]
    IOError(#[from] std::io::Error),

    #[error("TLS error")]
    TlsError(#[from] tokio_rustls::rustls::TLSError),

    #[error("parse config error")]
    ConfigError(#[from] toml::de::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}
