use crate::Message;
use futures::FutureExt;
use std::future::Future;
use tokio::sync::mpsc;
use tokio::sync::oneshot::{self, error::TryRecvError};

pub enum AsyncData<T> {
    NotStarted,
    InProgress(oneshot::Receiver<T>),
    Completed,
}

impl<T> Default for AsyncData<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AsyncData<T> {
    pub fn new() -> Self {
        Self::NotStarted
    }

    pub fn start<F>(&mut self, m_tx: &mpsc::UnboundedSender<Message>, fut: F)
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        assert!(matches!(self, Self::NotStarted));

        let (tx, rx) = oneshot::channel();
        let tx_fut = async move {
            tx.send(fut.await).ok().expect("Receiver closed");
        };
        let msg = Message::Execute(tx_fut.boxed());
        m_tx.send(msg).ok().expect("Message receiver closed");
        *self = Self::InProgress(rx);
    }

    pub fn poll(&mut self) -> Option<T> {
        if let Self::InProgress(rx) = self {
            match rx.try_recv() {
                Ok(val) => {
                    *self = Self::Completed;
                    Some(val)
                }
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Closed) => panic!(),
            }
        } else {
            None
        }
    }
}
