// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! CAR file chunking utilities
//!
//! See https://ipld.io/specs/transport/car/carv1/

use anyhow::{self, Context as AnyhowContext};
use cid::Cid;
use futures::{future, AsyncRead, AsyncWrite, Future, FutureExt, Stream, StreamExt};
use std::io::{Error as IoError, Result as IoResult};
use std::path::PathBuf;
use std::pin::{pin, Pin};
use std::task::{ready, Context, Poll};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use fvm_ipld_car::Error as CarError;
use fvm_ipld_car::{CarHeader, CarReader};

/// Take an existing CAR file and split it up into an output directory by creating
/// files with a limited size for each file.
///
/// The first (0th) file will be just the header, with the rest containing the "content" blocks.
pub async fn split(
    input_file: PathBuf,
    output_dir: PathBuf,
    max_size_bytes: usize,
) -> anyhow::Result<()> {
    let file = tokio::fs::File::open(input_file.clone())
        .await
        .with_context(|| format!("failed to open CAR file: {}", input_file.to_string_lossy()))?;

    let mut reader: CarReader<_> = CarReader::new_unchecked(file.compat())
        .await
        .context("failed to open CAR reader")?;

    let mut idx = 0;
    let mut next_output = || {
        let out = output_dir.join(idx.to_string());
        idx += 1;
        tokio::fs::File::create(out)
    };

    // TODO: Create a Writer that opens new files when the maximum is reached.
    let out = next_output().await.context("failed to create output")?;
    let mut out = out.compat_write();
    let mut out = Pin::new(&mut out);

    let header = CarHeader::new(reader.header.roots.clone(), reader.header.version);

    let block_streamer = BlockStreamer::new(&mut reader);
    // We shouldn't see errors when reading the CAR files, as we have written them ourselves,
    // but for piece of mind let's log any errors and move on.
    let mut block_streamer = block_streamer.filter_map(|res| match res {
        Ok(b) => future::ready(Some(b)),
        Err(e) => {
            tracing::warn!(error = e.to_string(), "CAR block failure");
            future::ready(None)
        }
    });

    // Copy the input CAR into an output CAR.
    header
        .write_stream_async(&mut out, &mut block_streamer)
        .await
        .context("failed to write CAR file")?;

    Ok(())
}

/// Stream the content blocks from a CAR reader.
struct BlockStreamer<'a, R> {
    reader: &'a mut CarReader<R>,
}

impl<'a, R> BlockStreamer<'a, R> {
    pub fn new(reader: &'a mut CarReader<R>) -> Self {
        Self { reader }
    }
}

impl<'a, R> Stream for BlockStreamer<'a, R>
where
    R: AsyncRead + Send + Unpin,
{
    type Item = Result<(Cid, Vec<u8>), CarError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next_block = self.reader.next_block().map(|res| match res {
            Ok(None) => None,
            Ok(Some(b)) => Some(Ok((b.cid, b.data))),
            Err(e) => Some(Err(e)),
        });

        pin!(next_block).poll(cx)
    }
}

type BoxedFutureFile = Pin<Box<dyn Future<Output = IoResult<tokio::fs::File>>>>;
type BoxedFile = Pin<Box<tokio_util::compat::Compat<tokio::fs::File>>>;
type StatePoll<T> = (ChunkWriterState, Poll<IoResult<T>>);

enum ChunkWriterState {
    Idle,
    Opening { out: BoxedFutureFile },
    Open { out: BoxedFile, written: usize },
    Closing { out: BoxedFile },
}

impl ChunkWriterState {
    fn ok<T>(self, value: T) -> StatePoll<T> {
        (self, Poll::Ready(Ok(value)))
    }

    fn err<T>(self, err: IoError) -> StatePoll<T> {
        (self, Poll::Ready(Err(err)))
    }

    fn pending<T>(self) -> StatePoll<T> {
        (self, Poll::Pending)
    }
}

/// Write a CAR file to chunks under an output directory:
/// 1. the first chunk is assumed to be just the header and goes into its own file
/// 2. subsequent blocks are assumed to be the contents and go into files with limited size
struct ChunkWriter {
    output_dir: PathBuf,
    max_size: usize,
    next_idx: usize,
    state: ChunkWriterState,
}

impl ChunkWriter {
    fn take_state(&mut self) -> ChunkWriterState {
        let mut state = ChunkWriterState::Idle;
        std::mem::swap(&mut self.state, &mut state);
        state
    }

    /// Replace the state with a new one, returning the poll result.
    fn poll_state<F, T>(self: &mut Pin<&mut Self>, f: F) -> Poll<IoResult<T>>
    where
        F: FnOnce(&mut Pin<&mut Self>, ChunkWriterState) -> StatePoll<T>,
    {
        let state = self.take_state();
        let (state, poll) = f(self, state);
        self.state = state;
        poll
    }

    /// Open the file, then do something with it.
    fn state_poll_open<F, T>(cx: &mut Context<'_>, mut out: BoxedFutureFile, f: F) -> StatePoll<T>
    where
        F: FnOnce(&mut Context<'_>, BoxedFile) -> StatePoll<T>,
    {
        use ChunkWriterState::*;

        match out.as_mut().poll(cx) {
            Poll::Pending => Opening { out }.pending(),
            Poll::Ready(Err(e)) => Idle.err(e),
            Poll::Ready(Ok(out)) => {
                let out = Box::pin(out.compat_write());
                f(cx, out)
            }
        }
    }

    /// Write to the open file.
    fn state_poll_write(
        cx: &mut Context<'_>,
        buf: &[u8],
        mut out: BoxedFile,
        sofar: usize,
    ) -> StatePoll<usize> {
        use ChunkWriterState::*;

        match out.as_mut().poll_write(cx, buf) {
            Poll::Pending => Open {
                out,
                written: sofar,
            }
            .pending(),
            Poll::Ready(Ok(written)) => Open {
                out,
                written: sofar + written,
            }
            .ok(written),
            Poll::Ready(Err(e)) => Open {
                out,
                written: sofar,
            }
            .err(e),
        }
    }

    /// Close the file.
    fn state_poll_close(cx: &mut Context<'_>, mut out: BoxedFile) -> StatePoll<()> {
        use ChunkWriterState::*;

        match out.as_mut().poll_close(cx) {
            Poll::Pending => Closing { out }.pending(),
            Poll::Ready(Err(e)) => Idle.err(e),
            Poll::Ready(Ok(())) => Idle.ok(()),
        }
    }

    /// Open the file then write to it.
    fn state_poll_open_write(
        cx: &mut Context<'_>,
        buf: &[u8],
        out: BoxedFutureFile,
    ) -> StatePoll<usize> {
        Self::state_poll_open(cx, out, |cx, out| Self::state_poll_write(cx, buf, out, 0))
    }

    /// Open the next file, then increment the index.
    fn next_file(&mut self) -> BoxedFutureFile {
        let out = self.output_dir.join(self.next_idx.to_string());
        self.next_idx += 1;
        Box::pin(tokio::fs::File::create(out))
    }
}

impl AsyncWrite for ChunkWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        use ChunkWriterState::*;
        self.poll_state(|this, state| match state {
            Idle => Self::state_poll_open_write(cx, buf, this.next_file()),
            Opening { out } => Self::state_poll_open_write(cx, buf, out),
            Open { out, written } => Self::state_poll_write(cx, buf, out, written),
            Closing { out } => {
                let (state, poll) = Self::state_poll_close(cx, out);
                if poll.is_ready() {
                    Self::state_poll_open_write(cx, buf, this.next_file())
                } else {
                    state.pending()
                }
            }
        })
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        use ChunkWriterState::*;

        self.poll_state(|this, state| match state {
            Idle => state.ok(()),
            Opening { out } => {
                // When we just opened this file, there is nothing to flush.
                Self::state_poll_open(cx, out, |_cx: &mut Context<'_>, out| {
                    Open { out, written: 0 }.ok(())
                })
            }
            Open { mut out, written } => match out.as_mut().poll_flush(cx) {
                Poll::Pending => Open { out, written }.pending(),
                Poll::Ready(Err(e)) => Open { out, written }.err(e),
                Poll::Ready(Ok(())) => {
                    // Close the file if either:
                    // a) we have written the header, or
                    // b) we exceeded the maximum file size.
                    // The flush is ensured by `fvm_ipld_car::util::ld_write` called by `CarHeader::write_stream_async` with the header.
                    // The file is closed here not in `poll_write` so we don't have torn writes where the varint showing the size is split from the data.
                    let close = this.next_idx == 1 || written >= this.max_size && this.max_size > 0;

                    if close {
                        Self::state_poll_close(cx, out)
                    } else {
                        Open { out, written }.ok(())
                    }
                }
            },
            Closing { out } => Self::state_poll_close(cx, out),
        })
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        use ChunkWriterState::*;

        self.poll_state(|_, state| match state {
            Idle => state.ok(()),
            Opening { out } => {
                Self::state_poll_open(cx, out, |cx, out| Self::state_poll_close(cx, out))
            }
            Open { out, .. } => Self::state_poll_close(cx, out),
            Closing { out } => Self::state_poll_close(cx, out),
        })
    }
}
