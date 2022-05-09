// Copyright 2022 The Engula Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![feature(get_mut_unchecked)]

extern crate core;

use std::net::ToSocketAddrs;

use engula_engine::{Db, DiskCache};
use tracing::info;

mod error;
pub use error::{Error, Result};

mod config;
pub use config::{Config, ConfigBuilder};

mod buffer;
pub use buffer::{ReadBuf, WriteBuf};

#[allow(dead_code)]
mod cmd;

#[allow(dead_code)]
mod frame;
use frame::{Error as FrameError, Frame};

mod parse;
use parse::{Parse, ParseError};

mod connection;
use connection::Connection;

mod server;
mod shutdown;

pub use async_trait::async_trait;
use monoio::net::TcpListener;

pub fn run(config: Config) -> Result<()> {
    let mut rt = monoio::RuntimeBuilder::new()
        .with_entries(32768)
        .enable_timer()
        .build()
        .unwrap();
    rt.block_on(async {
        let disk_cache = DiskCache::new(&config.root, config.disk_opts.clone()).await?;
        let db = Db::new(config.max_memory, disk_cache);

        // Resolve & Bind a TCP listener
        let addr = config.addr.to_socket_addrs()?.next().unwrap();
        let listener = TcpListener::bind(addr)?;
        info!("bind address {}", addr);

        server::run(db, listener, config).await;
        Ok::<(), Error>(())
    })?;

    Ok(())
}
