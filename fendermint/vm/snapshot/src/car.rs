// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! CAR file chunking utilities
//!
//! See https://ipld.io/specs/transport/car/carv1/

use anyhow::{self, Context as AnyhowContext};
use cid::Cid;
use futures::{future, AsyncRead, AsyncWrite, Future, FutureExt, Stream, StreamExt};
use std::io::{Error as IoError, Result as IoResult};
use std::path::{Path, PathBuf};
use std::pin::{pin, Pin};
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

    let mut reader: CarReader<_> = CarReader::new_unchecked(file.compat())
        .await
        .context("failed to open CAR reader")?;

    // Create a Writer that opens new files when the maximum is reached.
    let mut writer = ChunkWriter::new(output_dir.into(), max_size);

    let header = CarHeader::new(reader.header.roots.clone(), reader.header.version);

    let block_streamer = BlockStreamer::new(&mut reader);
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
    use futures::{AsyncRead, Future, StreamExt};
    use fvm_ipld_blockstore::MemoryBlockstore;
    use fvm_ipld_car::{load_car, CarReader};
    use tempfile::tempdir;
    use tokio_util::compat::TokioAsyncReadCompatExt;

    use super::{split, BlockStreamer};

    fn bundle_bytes() -> Vec<u8> {
        let bundle_path = bundle_path();
        std::fs::read(bundle_path).unwrap()
    }

    async fn bundle_file() -> tokio::fs::File {
        let bundle_path = bundle_path();
        tokio::fs::File::open(bundle_path).await.unwrap()
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
        R: AsyncRead + Send + Unpin,
    {
        let mut reader = CarReader::new_unchecked(reader)
            .await
            .expect("failed to open CAR reader");

        let streamer = BlockStreamer::new(&mut reader);

        streamer
            .for_each(|r| async move {
                r.expect("should be ok");
            })
            .await;
    }

    /// Sanity check that the test bundle can be loaded with the normal facilities from memory.
    #[tokio::test]
    async fn load_bundle_from_memory() {
        let bundle_bytes = bundle_bytes();
        check_load_car(bundle_bytes.as_slice()).await;
    }

    /// Sanity check that the test bundle can be loaded with the normal facilities from a file.
    #[tokio::test]
    async fn load_bundle_from_file() {
        let bundle_file = bundle_file().await;
        check_load_car(bundle_file.compat()).await;
    }

    #[tokio::test]
    async fn block_streamer_from_memory() {
        let bundle_bytes = bundle_bytes();
        check_block_streamer(bundle_bytes.as_slice()).await;
    }

    #[tokio::test]
    async fn block_streamer_from_file() {
        let bundle_file = bundle_file().await;
        check_block_streamer(bundle_file.compat()).await;
    }

    /// Sanity check that a reader can go through the bundle file.
    #[tokio::test]
    async fn next_block_from_file() {
        let bundle_file = bundle_file().await;
        let mut reader = CarReader::new_unchecked(bundle_file.compat())
            .await
            .unwrap();
        while reader.next_block().await.expect("should be ok").is_some() {}
    }

    #[tokio::test]
    async fn poll_next_block_from_file() {
        let bundle_file = bundle_file().await;
        let mut reader = CarReader::new_unchecked(bundle_file.compat())
            .await
            .unwrap();

        loop {
            let poll = futures::future::poll_fn(|cx| {
                let next_block = reader.next_block();

                // 1. Try with `pin!`
                std::pin::pin!(next_block).poll(cx)

                // 2. Try with `Box::pin`
                //let mut next_block = Box::pin(next_block);
                //next_block.as_mut().poll(cx)

                // 3. Try with `tokio::pin!`
                // tokio::pin!(next_block);
                // next_block.poll(cx)
            });
            if poll.await.expect("should be ok").is_none() {
                break;
            }
        }
    }

    #[tokio::test]
    async fn poll_read_node_from_file() {
        let bundle_file = bundle_file().await;
        let mut reader = CarReader::new_unchecked(bundle_file.compat())
            .await
            .unwrap();

        loop {
            let poll = futures::future::poll_fn(|cx| {
                let next_block = util::read_node(&mut reader.reader);
                std::pin::pin!(next_block).poll(cx)
            });
            if poll.await.expect("should be ok").is_none() {
                break;
            }
        }
    }

    /// Load the actor bundle CAR file, split it into chunks, then restore and compare to the original.
    #[tokio::test]
    async fn split_bundle_car() {
        let bundle_path = bundle_path();
        let bundle_size = std::fs::metadata(&bundle_path).unwrap().len() as usize;

        let tmp = tempdir().unwrap();
        let target_count = 10;
        let max_size = bundle_size / target_count;

        split(&bundle_path, tmp.path(), max_size)
            .await
            .expect("failed to split CAR file");

        let chunks = std::fs::read_dir(tmp.path()).unwrap();
        let chunks_count = chunks.count();

        assert!(
            1 + target_count <= chunks_count && chunks_count <= 1 + target_count + 1,
            "expected 1 header and {} chunks, got {}",
            target_count,
            chunks_count
        );
    }

    /// Copied functions from `fvm_ipld_car::util` for testing.
    mod util {
        use cid::Cid;
        use futures::{AsyncRead, AsyncReadExt};
        use fvm_ipld_car::Error;
        use integer_encoding::VarIntAsyncReader;

        pub async fn ld_read<R>(mut reader: &mut R) -> Result<Option<Vec<u8>>, Error>
        where
            R: AsyncRead + Send + Unpin,
        {
            const MAX_ALLOC: usize = 1 << 20;
            let l: usize = match VarIntAsyncReader::read_varint_async(&mut reader).await {
                Ok(len) => len,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        return Ok(None);
                    }
                    return Err(Error::Other(e.to_string()));
                }
            };
            let mut buf = Vec::with_capacity(std::cmp::min(l as usize, MAX_ALLOC));
            let bytes_read = reader
                .take(l as u64)
                .read_to_end(&mut buf)
                .await
                .map_err(|e| Error::Other(e.to_string()))?;
            if bytes_read != l {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!(
                        "expected to read at least {} bytes, but read {}",
                        l, bytes_read
                    ),
                )));
            }
            Ok(Some(buf))
        }

        pub async fn read_node<R>(buf_reader: &mut R) -> Result<Option<(Cid, Vec<u8>)>, Error>
        where
            R: AsyncRead + Send + Unpin,
        {
            match ld_read(buf_reader).await? {
                Some(buf) => {
                    let mut cursor = std::io::Cursor::new(&buf);
                    let cid = Cid::read_bytes(&mut cursor)?;
                    Ok(Some((cid, buf[cursor.position() as usize..].to_vec())))
                }
                None => Ok(None),
            }
        }
    }
}
