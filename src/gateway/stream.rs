//! The ordered event stream behind [`Gateway::frames`](super::Gateway::frames).
//!
//! One channel carries every inbound frame and every status change, in the
//! order the supervisor saw them. The sending side never waits on the
//! consumer: a bounded channel that is full drops the event, counts it, and
//! reports the count as [`GatewayEvent::Lagged`] as soon as there is room.

use std::time::Duration;

use tokio::sync::mpsc::{self, error::TrySendError};

/// One event on the [`Gateway::frames`](super::Gateway::frames) stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayEvent {
    /// An inbound frame, as received (decompressed when it arrived as a
    /// binary frame). `s` is Discord's sequence number, not the library's.
    Frame { op: u8, s: Option<u64>, t: Option<String>, raw: String },
    /// The connection status changed. Never repeated for an unchanged
    /// status.
    Status { status: GatewayStatus, detail: Option<String> },
    /// `dropped` events did not fit a bounded channel. Only sent on a
    /// [`FrameBuffer::Bounded`] stream.
    Lagged { dropped: u64 },
}

/// The connection status reported on the event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayStatus {
    /// A socket is being opened, or is open and has not yet reached READY or
    /// RESUMED.
    Connecting,
    /// READY or RESUMED arrived on the current connection.
    Live,
    /// Discord rejected the token (close 4004). Not terminal by itself: the
    /// close policy decides whether [`GatewayStatus::Connecting`] or
    /// [`GatewayStatus::Stopped`] follows.
    AuthFailed,
    /// The supervisor has exited. Always the last event; the channel closes
    /// after it.
    Stopped,
}

/// How [`Gateway::frames`](super::Gateway::frames) buffers events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameBuffer {
    /// Never drops an event. Memory grows while the consumer falls behind.
    Unbounded,
    /// Holds at most this many events (minimum 1). Events beyond it are
    /// dropped, and counted in a [`GatewayEvent::Lagged`].
    Bounded(usize),
}

/// The receiving end of [`Gateway::frames`](super::Gateway::frames).
#[derive(Debug)]
pub struct GatewayEvents {
    rx: Receiver,
}

#[derive(Debug)]
enum Receiver {
    Bounded(mpsc::Receiver<GatewayEvent>),
    Unbounded(mpsc::UnboundedReceiver<GatewayEvent>),
}

impl GatewayEvents {
    /// The next event, or `None` once the supervisor has stopped and every
    /// event before that has been received.
    pub async fn recv(&mut self) -> Option<GatewayEvent> {
        match &mut self.rx {
            Receiver::Bounded(rx) => rx.recv().await,
            Receiver::Unbounded(rx) => rx.recv().await,
        }
    }

    /// The next event if one is ready.
    pub fn try_recv(&mut self) -> Result<GatewayEvent, mpsc::error::TryRecvError> {
        match &mut self.rx {
            Receiver::Bounded(rx) => rx.try_recv(),
            Receiver::Unbounded(rx) => rx.try_recv(),
        }
    }
}

/// How long the final `Stopped` may wait for room in a full bounded channel.
const STOPPED_GRACE: Duration = Duration::from_secs(1);

/// The sending end, owned by the supervisor through a shared slot.
#[derive(Debug)]
pub(crate) struct EventSink {
    tx: Sender,
    dropped: u64,
    last_status: Option<GatewayStatus>,
}

#[derive(Debug)]
enum Sender {
    Bounded(mpsc::Sender<GatewayEvent>),
    Unbounded(mpsc::UnboundedSender<GatewayEvent>),
}

impl EventSink {
    pub(crate) fn channel(buffer: FrameBuffer) -> (Self, GatewayEvents) {
        let (tx, rx) = match buffer {
            FrameBuffer::Unbounded => {
                let (tx, rx) = mpsc::unbounded_channel();
                (Sender::Unbounded(tx), Receiver::Unbounded(rx))
            }
            FrameBuffer::Bounded(capacity) => {
                let (tx, rx) = mpsc::channel(capacity.max(1));
                (Sender::Bounded(tx), Receiver::Bounded(rx))
            }
        };
        (Self { tx, dropped: 0, last_status: None }, GatewayEvents { rx })
    }

    /// Deliver `event` without waiting. On a full bounded channel the event
    /// is dropped and counted.
    pub(crate) fn emit(&mut self, event: GatewayEvent) {
        match &self.tx {
            Sender::Unbounded(tx) => {
                let _ = tx.send(event);
            }
            Sender::Bounded(tx) => {
                if self.dropped > 0 {
                    match tx.try_send(GatewayEvent::Lagged { dropped: self.dropped }) {
                        Ok(()) => self.dropped = 0,
                        Err(TrySendError::Full(_)) => {
                            self.dropped += 1;
                            return;
                        }
                        Err(TrySendError::Closed(_)) => return,
                    }
                }
                if let Err(TrySendError::Full(_)) = tx.try_send(event) {
                    self.dropped += 1;
                }
            }
        }
    }

    /// Report `status` unless it is the status last reported.
    pub(crate) fn status(&mut self, status: GatewayStatus, detail: Option<String>) {
        if self.last_status == Some(status) {
            return;
        }
        self.last_status = Some(status);
        self.emit(GatewayEvent::Status { status, detail });
    }

    /// Report [`GatewayStatus::Stopped`] and close the channel. A full bounded
    /// channel gets a short grace period, so the terminal event is not lost
    /// to a consumer that is merely slow.
    pub(crate) async fn finish(mut self, detail: Option<String>) {
        let stopped = GatewayEvent::Status { status: GatewayStatus::Stopped, detail };
        match &self.tx {
            Sender::Unbounded(tx) => {
                let _ = tx.send(stopped);
            }
            Sender::Bounded(tx) => {
                let tx = tx.clone();
                let send = async {
                    if self.dropped > 0 && tx.send(GatewayEvent::Lagged { dropped: self.dropped }).await.is_ok() {
                        self.dropped = 0;
                    }
                    let _ = tx.send(stopped).await;
                };
                let _ = tokio::time::timeout(STOPPED_GRACE, send).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(n: u64) -> GatewayEvent {
        GatewayEvent::Frame { op: 0, s: Some(n), t: Some("X".to_string()), raw: n.to_string() }
    }

    fn drain(events: &mut GatewayEvents) -> Vec<GatewayEvent> {
        std::iter::from_fn(|| events.try_recv().ok()).collect()
    }

    #[test]
    fn unbounded_keeps_everything_in_order() {
        let (mut sink, mut events) = EventSink::channel(FrameBuffer::Unbounded);
        for n in 0..1000 {
            sink.emit(frame(n));
        }
        let got = drain(&mut events);
        assert_eq!(got.len(), 1000);
        assert_eq!(got[999], frame(999));
    }

    #[test]
    fn bounded_counts_what_it_drops_and_reports_it_when_room_returns() {
        let (mut sink, mut events) = EventSink::channel(FrameBuffer::Bounded(2));
        for n in 0..5 {
            sink.emit(frame(n));
        }
        assert_eq!(drain(&mut events), [frame(0), frame(1)]);
        sink.emit(frame(5));
        assert_eq!(drain(&mut events), [GatewayEvent::Lagged { dropped: 3 }, frame(5)]);
    }

    #[test]
    fn lagged_that_does_not_fit_keeps_counting() {
        let (mut sink, mut events) = EventSink::channel(FrameBuffer::Bounded(1));
        sink.emit(frame(0));
        sink.emit(frame(1));
        sink.emit(frame(2));
        assert_eq!(drain(&mut events), [frame(0)]);
        sink.emit(frame(3));
        // Lagged took the only slot, so frame 3 is dropped and counted.
        assert_eq!(drain(&mut events), [GatewayEvent::Lagged { dropped: 2 }]);
        sink.emit(frame(4));
        assert_eq!(drain(&mut events), [GatewayEvent::Lagged { dropped: 1 }]);
    }

    #[test]
    fn status_is_reported_only_on_change() {
        let (mut sink, mut events) = EventSink::channel(FrameBuffer::Unbounded);
        sink.status(GatewayStatus::Connecting, None);
        sink.status(GatewayStatus::Connecting, None);
        sink.status(GatewayStatus::Live, None);
        sink.status(GatewayStatus::Connecting, None);
        let statuses: Vec<_> = drain(&mut events)
            .into_iter()
            .map(|e| match e {
                GatewayEvent::Status { status, .. } => status,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(statuses, [GatewayStatus::Connecting, GatewayStatus::Live, GatewayStatus::Connecting]);
    }

    #[tokio::test]
    async fn finish_sends_stopped_last_and_closes_the_channel() {
        let (mut sink, mut events) = EventSink::channel(FrameBuffer::Bounded(4));
        sink.emit(frame(0));
        sink.finish(Some("shutdown".to_string())).await;
        assert_eq!(events.recv().await, Some(frame(0)));
        assert_eq!(events.recv().await, Some(GatewayEvent::Status { status: GatewayStatus::Stopped, detail: Some("shutdown".to_string()) }));
        assert_eq!(events.recv().await, None);
    }

    #[tokio::test]
    async fn finish_waits_briefly_for_room() {
        let (mut sink, mut events) = EventSink::channel(FrameBuffer::Bounded(1));
        sink.emit(frame(0));
        sink.emit(frame(1));
        let finish = tokio::spawn(sink.finish(None));
        assert_eq!(events.recv().await, Some(frame(0)));
        assert_eq!(events.recv().await, Some(GatewayEvent::Lagged { dropped: 1 }));
        assert_eq!(events.recv().await, Some(GatewayEvent::Status { status: GatewayStatus::Stopped, detail: None }));
        assert_eq!(events.recv().await, None);
        finish.await.unwrap();
    }
}
