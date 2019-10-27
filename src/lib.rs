#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use futures::future::Future as _;
use futures::stream::Stream as _;
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

pub type Result<T> = std::result::Result<T, Error>;

pub struct Resizer {
    winches:
        Box<dyn futures::stream::Stream<Item = (), Error = Error> + Send>,
    sent_initial_size: bool,
}

impl Resizer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Resizer {
    fn default() -> Self {
        let winches = tokio_signal::unix::Signal::new(
            tokio_signal::unix::libc::SIGWINCH,
        )
        .flatten_stream()
        .map(|_| ())
        .context(SigWinchHandler);
        Self {
            winches: Box::new(winches),
            sent_initial_size: false,
        }
    }
}

#[must_use = "streams do nothing unless polled"]
impl futures::stream::Stream for Resizer {
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

fn term_size() -> Result<(u16, u16)> {
    if let Some((cols, rows)) = term_size::dimensions() {
        Ok((
            rows.try_into().context(InvalidTerminalSize)?,
            cols.try_into().context(InvalidTerminalSize)?,
        ))
    } else {
        Err(Error::GetTerminalSize)
    }
}
