use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendError {
    Full,
    Disconnected,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TrySendError<T> {
    pub(crate) err: SendError,
    pub(crate) val: T,
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            SendError::Full => write!(f, "send failed because channel is full"),
            SendError::Disconnected => write!(f, "send failed because receiver is gone"),
        }
    }
}

impl std::error::Error for SendError {}

impl SendError {
    pub fn is_full(&self) -> bool {
        matches!(&self, SendError::Full)
    }

    pub fn is_disconnected(&self) -> bool {
        matches!(&self, SendError::Disconnected)
    }
}

impl<T> fmt::Debug for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrySendError")
            .field("kind", &self.err)
            .finish()
    }
}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.err.fmt(f)
    }
}

impl<T: core::any::Any> std::error::Error for TrySendError<T> {}

impl<T> TrySendError<T> {
    pub fn is_full(&self) -> bool {
        self.err.is_full()
    }

    pub fn is_disconnected(&self) -> bool {
        self.err.is_disconnected()
    }

    pub fn into_inner(self) -> T {
        self.val
    }

    pub fn into_send_error(self) -> SendError {
        self.err
    }
}

#[derive(Debug)]
pub enum TryRecvError {
    Empty,
    Disconnected,
}

impl fmt::Display for TryRecvError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryRecvError::Empty => "receiving on an empty channel".fmt(fmt),
            TryRecvError::Disconnected => "receiving on a closed channel".fmt(fmt),
        }
    }
}

impl std::error::Error for TryRecvError {}
