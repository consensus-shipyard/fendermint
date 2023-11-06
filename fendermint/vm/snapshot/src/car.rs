// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
//! CAR file chunking utilities
//!
//! See https://ipld.io/specs/transport/car/carv1/

use anyhow::{self, Context};
use cid::Cid;
use futures::{future, AsyncRead, AsyncWrite, Future, FutureExt, Stream, StreamExt};
use std::path::PathBuf;
use std::pin::{pin, Pin};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use fvm_ipld_car::{Block, Error as CarError};
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

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let next_block = self.reader.next_block().map(|res| match res {
            Ok(None) => None,
            Ok(Some(b)) => Some(Ok((b.cid, b.data))),
            Err(e) => Some(Err(e)),
        });

        let next_block = pin!(next_block);

        next_block.poll(cx)
    }
}

/// Write a CAR file to chunks under an output directory:
/// 1. the first chunk is assumed to be just the header and goes into its own file
/// 2. subsequent blocks are assumed to be the contents and go into files with limited size
struct ChunkWriter {
    output_dir: PathBuf,
    max_size_bytes: usize,
    next_idx: usize,
    open_file: Option<tokio_util::compat::Compat<tokio::fs::File>>,
}

// impl AsyncWrite for ChunkWriter {}
