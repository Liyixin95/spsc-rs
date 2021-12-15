//! A single-producer, single-consumer queue for sending values between
//! asynchronous tasks.
//!
//! `spsc-rs` is almost the drop in replacement for `tokio`'s `mpsc` channel when
//! you don't need the 'sender' to be cloneable. Here is the similarity and difference.
//!
//! # Similarity
//!
//! ## Separate Handles
//!
//! The channel creation provides separate handles for both [`bounded`] and [`unbounded`] channel.
//!
//! ## Disconnection
//!
//! When the [`Sender`] handle has been dropped, the `Receiver::poll` returns `Ok(Ready(None))`.
//!
//! When the [`Receiver`] handle is dropped, all further attempts to send will
//! result in an error.
//!
//! ## Clean Shutdown
//!
//! The [`Receiver`] provides `close` method to prevent further message from [`Sender`].
//! When closed, the [`Receiver`] can consume the remaining messages in the channel.
//!
//! # Difference
//!
//! ## `Sender` can not be cloned
//!
//! There can only exist one [`Sender`] and one [`Receiver`], then we can achieve higher performance.
//!
//! ## Underlying storage
//!
//! For [`unbounded`] channel, the underlying storage is a linked chunk queue, which is
//! similar to `tokio`.
//!
//! For [`bounded`] channel, we use a ring buffer as the inner storage. So when you use [`bounded`]
//! function to create a channel, the actual channel's size may be bigger then the number you passed in.
//! Because ring buffer need it's size to be power of two, and also need reserve one slot. This behavior is
//! the same as [`VecDeque`].
//!
//! If you don't want the requirement of power of tow, you can use [`exact_channel`] to create channel, which
//! will not expand the underlying buffer's size to power of two, but will sacrifice a little performance.
//!
//! ## Batch operation
//!
//! Both send and receive support batch operation. You can use [`start_send`] to fill an item to the channel without
//! notifying the receiver. When the channel is full, [`start_send`] will return `Err(TrySendErr::Full)`, and you
//! should use [`flush`] to notify the receiver to consume messages.
//!
//! For the receiver, you can use [`try_recv`] to fetch an item from the channel. When the channel is empty, [`try_recv`]
//! will return `Err(TryReceiveErr::Empty)`, and you should use [`want_recv`] to notify the sender to send more message.
//!
//! ## `Stream` trait
//!
//! The [`Receiver`] has implemented the `Stream` trait, but you still need to use [`SenderWrapper`] for `Sink` trait.
//!
//! [`Sender`]: crate::bounded::Sender
//! [`Receiver`]: crate::bounded::Receiver
//! [`SenderWrapper`]: crate::bounded::wrapper::SenderWrapper
//! [`exact_channel`]: crate::exact_channel
//! [`try_recv`]: crate::bounded::Receiver::try_recv
//! [`start_send`]: crate::bounded::Sender::start_send
//! [`flush`]: crate::bounded::Sender::flush
//! [`want_recv`]: crate::bounded::Receiver::want_recv
//! [`VecDeque`]: std::collections::VecDeque

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
