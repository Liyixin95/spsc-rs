#[macro_use]
mod loom;

mod atomic_waker;

pub mod error;

mod bounded;

pub use self::bounded::{
    channel, exact_channel, wrapper::SenderWrapper, ExactReceiver, ExactSender, P2Receiver,
    P2Sender,
};

mod unbounded;

pub use self::unbounded::{
    unbounded_channel, wrapper::UnboundedSenderWrapper, UnboundedReceiver, UnboundedSender,
};
