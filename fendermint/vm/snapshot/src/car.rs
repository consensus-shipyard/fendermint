// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! CAR file chunking utilities
//!
//! See https://ipld.io/specs/transport/car/carv1/

use anyhow::{self, Context as AnyhowContext};
use cid::Cid;
use futures::{future, AsyncRead, AsyncWrite, Future, Stream, StreamExt};
use std::io::{Error as IoError, Result as IoResult};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use fvm_ipld_car::Error as CarError;
use fvm_ipld_car::{CarHeader, CarReader};

/// Take an existing CAR file and split it up into an output directory by creating
/// files with a limited size for each file.
///
/// The first (0th) file will be just the header, with the rest containing the "content" blocks.
pub async fn split(input_file: &Path, output_dir: &Path, max_size: usize) -> anyhow::Result<()> {
    let file = tokio::fs::File::open(input_file)
        .await
        .with_context(|| format!("failed to open CAR file: {}", input_file.to_string_lossy()))?;

    let reader: CarReader<_> = CarReader::new_unchecked(file.compat())
        .await
        .context("failed to open CAR reader")?;

    // Create a Writer that opens new files when the maximum is reached.
    let mut writer = ChunkWriter::new(output_dir.into(), max_size);

    let header = CarHeader::new(reader.header.roots.clone(), reader.header.version);

    let block_streamer = BlockStreamer::new(reader);
    // We shouldn't see errors when reading the CAR files, as we have written them ourselves,
    // but for piece of mind let's log any errors and move on.
    let mut block_streamer = block_streamer.filter_map(|res| match res {
        Ok(b) => future::ready(Some(b)),
        Err(e) => {
            // TODO: It would be better to stop if there are errors.
            tracing::warn!(
                error = e.to_string(),
                file = input_file.to_string_lossy().to_string(),
                "CAR block failure"
            );
            future::ready(None)
        }
    });

    // Copy the input CAR into an output CAR.
    header
        .write_stream_async(&mut writer, &mut block_streamer)
        .await
        .context("failed to write CAR file")?;

    Ok(())
}

type BlockStreamerItem = Result<(Cid, Vec<u8>), CarError>;
type BlockStreamerRead<R> = (CarReader<R>, Option<BlockStreamerItem>);
type BlockStreamerReadFuture<R> = Pin<Box<dyn Future<Output = BlockStreamerRead<R>>>>;

enum BlockStreamerState<R> {
    Idle(CarReader<R>),
    Reading(BlockStreamerReadFuture<R>),
}

/// Stream the content blocks from a CAR reader.
struct BlockStreamer<R> {
    state: Option<BlockStreamerState<R>>,
}

impl<R> BlockStreamer<R>
where
    R: AsyncRead + Send + Unpin,
{
    pub fn new(reader: CarReader<R>) -> Self {
        Self {
            state: Some(BlockStreamerState::Idle(reader)),
        }
    }

    async fn next_block(mut reader: CarReader<R>) -> BlockStreamerRead<R> {
        let res = reader.next_block().await;
        let out = match res {
            Err(e) => Some(Err(e)),
            Ok(Some(b)) => Some(Ok((b.cid, b.data))),
            Ok(None) => None,
        };
        (reader, out)
    }

    fn poll_next_block(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut next_block: BlockStreamerReadFuture<R>,
    ) -> Poll<Option<BlockStreamerItem>> {
        use BlockStreamerState::*;

        match next_block.as_mut().poll(cx) {
            Poll::Pending => {
                self.state = Some(Reading(next_block));
                Poll::Pending
            }
            Poll::Ready((reader, out)) => {
                self.state = Some(Idle(reader));
                Poll::Ready(out)
            }
        }
    }
}

impl<R> Stream for BlockStreamer<R>
where
    R: AsyncRead + Send + Unpin + 'static,
{
    type Item = BlockStreamerItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use BlockStreamerState::*;

        match self.state.take() {
            None => Poll::Ready(None),
            Some(Idle(reader)) => {
                let next_block = Self::next_block(reader);
                let next_block = Box::pin(next_block);
                self.poll_next_block(cx, next_block)
            }
            Some(Reading(next_block)) => self.poll_next_block(cx, next_block),
        }
    }
}

type BoxedFutureFile = Pin<Box<dyn Future<Output = IoResult<tokio::fs::File>> + Send + 'static>>;
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
    pub fn new(output_dir: PathBuf, max_size: usize) -> Self {
        Self {
            output_dir,
            max_size,
            next_idx: 0,
            state: ChunkWriterState::Idle,
        }
    }

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

#[cfg(test)]
mod tests {

    use fendermint_vm_interpreter::fvm::bundle::bundle_path;
    use futures::{AsyncRead, StreamExt};
    use fvm_ipld_blockstore::MemoryBlockstore;
    use fvm_ipld_car::{load_car, CarReader};
    use tempfile::tempdir;
    use tokio_util::compat::TokioAsyncReadCompatExt;

    use super::{split, BlockStreamer};

    async fn bundle_file() -> tokio_util::compat::Compat<tokio::fs::File> {
        let bundle_path = bundle_path();
        tokio::fs::File::open(bundle_path).await.unwrap().compat()
    }

    /// Check that a CAR file can be loaded from a byte reader.
    async fn check_load_car<R>(reader: R)
    where
        R: AsyncRead + Send + Unpin,
    {
        let store = MemoryBlockstore::new();
        load_car(&store, reader).await.expect("failed to load CAR");
    }

    /// Check that a CAR file can be streamed without errors.
    async fn check_block_streamer<R>(reader: R)
    where
        R: AsyncRead + Send + Unpin + 'static,
    {
        let reader = CarReader::new_unchecked(reader)
            .await
            .expect("failed to open CAR reader");

        let streamer = BlockStreamer::new(reader);

        streamer
            .for_each(|r| async move {
                r.expect("should be ok");
            })
            .await;
    }

    /// Sanity check that the test bundle can be loaded with the normal facilities from a file.
    #[tokio::test]
    async fn load_bundle_from_file() {
        let bundle_file = bundle_file().await;
        check_load_car(bundle_file).await;
    }

    #[tokio::test]
    async fn block_streamer_from_file() {
        let bundle_file = bundle_file().await;
        check_block_streamer(bundle_file).await;
    }

    /// Load the actor bundle CAR file, split it into chunks, then restore and compare to the original.
    #[tokio::test]
    async fn split_bundle_car() {
        let bundle_path = bundle_path();
        let bundle_bytes = std::fs::read(&bundle_path).unwrap();

        let tmp = tempdir().unwrap();
        let target_count = 10;
        let max_size = bundle_bytes.len() / target_count;

        split(&bundle_path, tmp.path(), max_size)
            .await
            .expect("failed to split CAR file");

        let mut chunks = std::fs::read_dir(tmp.path())
            .unwrap()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        chunks.sort_unstable_by_key(|c| c.path().to_string_lossy().to_string());

        let chunks = chunks
            .into_iter()
            .map(|c| {
                let chunk_size = std::fs::metadata(&c.path()).unwrap().len() as usize;
                (c, chunk_size)
            })
            .collect::<Vec<_>>();

        let chunks_bytes = chunks.iter().fold(Vec::new(), |mut acc, (c, _)| {
            let bz = std::fs::read(c.path()).unwrap();
            acc.extend(bz);
            acc
        });

        assert!(
            1 < chunks.len() && chunks.len() <= 1 + target_count,
            "expected 1 header and max {} chunks, got {}",
            target_count,
            chunks.len()
        );

        assert!(chunks[0].1 < 100, "header is small");
        assert_eq!(chunks_bytes.len(), bundle_bytes.len());
        assert_eq!(chunks_bytes[0..100], bundle_bytes[0..100]);
    }
}
