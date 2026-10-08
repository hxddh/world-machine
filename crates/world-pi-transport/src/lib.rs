//! Talking to a local pi program over its RPC mode: a request per prompt
//! (`ProcessPiRpcTransport`) or one pi kept running (`PersistentPiRpcTransport`),
//! held to a deadline and cancellable, and the reading of its event
//! stream. It knows nothing of any World; the `AgentRuntime` adapter that
//! asks pi to choose among a World's actions is `world-pi-rpc`, and the
//! World voice asks pi through this crate alone.

#![forbid(unsafe_code)]

mod protocol;
mod transport;

pub use protocol::{parse_decision, PiRpcEventParser, PiRpcProtocolError};
pub use transport::{
    PersistentPiRpcTransport, PiCommand, PiRpcTransport, PiRpcTransportError,
    ProcessPiRpcTransport, MOST_REQUESTS,
};
