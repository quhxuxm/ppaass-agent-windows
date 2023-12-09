use ppaass_codec::error::{DecoderError, EncoderError};
use ppaass_protocol::error::ProtocolError;
use std::io::Error as StdIoError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Timeout error happen: {0}")]
    Timeout(u64),
    #[error("I/O error happen: {0:?}")]
    Io(#[from] StdIoError),
    #[error("Proxy decoder error happen: {0:?}")]
    DecoderProxyEdge(#[from] DecoderError),
    #[error("Proxy encoder error happen: {0:?}")]
    EncoderProxyEdge(#[from] EncoderError),

    #[error("Proxy decoder error happen: {0:?}")]
    DecoderClient(String),
    #[error("Proxy encoder error happen: {0:?}")]
    EncoderClient(String),

    #[error("Protocol error happen: {0:?}")]
    Protocol(#[from] ProtocolError),
    #[error("Other error happen: {0}")]
    Other(String),
}
