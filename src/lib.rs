#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use futures::future::Future as _;
use futures::stream::Stream as _;
use snafu::futures01::FutureExt as _;
use snafu::futures01::StreamExt as _;
use snafu::ResultExt as _;
use std::convert::TryInto as _;

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("failed to get terminal size"))]
    GetTerminalSize,

    #[snafu(display("invalid terminal size found: {}", source))]
    InvalidTerminalSize { source: std::num::TryFromIntError },

    #[snafu(display("SIGWINCH handler failed: {}", source))]
    SigWinchHandler { source: std::io::Error },
}

pub fn resizes() -> ResizeFuture {
    ResizeFuture::default()
}

pub struct ResizeFuture {
    stream_fut: Box<
        dyn futures::future::Future<Item = ResizeStream, Error = Error>
            + Send,
    >,
}

impl Default for ResizeFuture {
    fn default() -> Self {
        let stream_fut = tokio_signal::unix::Signal::new(
            tokio_signal::unix::libc::SIGWINCH,
        )
        .context(SigWinchHandler)
        .and_then(|stream| {
            futures::future::ok(ResizeStream {
                winches: Box::new(
                    stream.map(|_| ()).context(SigWinchHandler),
                ),
                sent_initial_size: false,
            })
        });
        Self {
            stream_fut: Box::new(stream_fut),
        }
    }
}

#[must_use = "streams do nothing unless polled"]
impl futures::future::Future for ResizeFuture {
    type Item = ResizeStream;
    type Error = Error;

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        self.stream_fut.poll()
    }
}

pub struct ResizeStream {
    winches:
        Box<dyn futures::stream::Stream<Item = (), Error = Error> + Send>,
    sent_initial_size: bool,
}

#[must_use = "streams do nothing unless polled"]
impl futures::stream::Stream for ResizeStream {
    type Item = (u16, u16);
    type Error = Error;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        if !self.sent_initial_size {
            self.sent_initial_size = true;
            return Ok(futures::Async::Ready(Some(term_size()?)));
        }
        futures::try_ready!(self.winches.poll());
        Ok(futures::Async::Ready(Some(term_size()?)))
    }
}

fn term_size() -> Result<(u16, u16), Error> {
    if let Some((cols, rows)) = term_size::dimensions() {
        Ok((
            rows.try_into().context(InvalidTerminalSize)?,
            cols.try_into().context(InvalidTerminalSize)?,
        ))
    } else {
        Err(Error::GetTerminalSize)
    }
}
