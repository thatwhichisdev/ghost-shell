mod protocol;
pub mod unix;

pub use protocol::*;
pub use unix::{
    NiriClient, NiriClientError, NiriClientPool, NiriStream, StreamError, client,
    connect, events, send, stream,
};
