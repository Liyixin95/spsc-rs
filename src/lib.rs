mod atomic_waker;

pub mod error;

mod loom;

mod bounded;

pub use self::bounded::{
    channel, exact_channel, wrapper::SenderWrapper, ExactReceiver, ExactSender, P2Receiver,
    P2Sender,
};
