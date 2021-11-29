// Copyright 2021 The Engula Authors.
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

use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use super::async_trait;

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl From<u64> for Timestamp {
    fn from(t: u64) -> Self {
        Timestamp(t)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub ts: Timestamp,
    pub data: Vec<u8>,
}

/// An interface to manipulate a stream.
#[async_trait]
pub trait Stream {
    type Error;
    type EventStream: futures::Stream<Item = Result<Event, Self::Error>> + Unpin;

    /// Reads events since a timestamp (inclusive).
    async fn read_events(&self, ts: Timestamp) -> Result<Self::EventStream, Self::Error>;

    /// Appends an event.
    async fn append_event(&self, event: Event) -> Result<(), Self::Error>;

    /// Releases events up to a timestamp (exclusive).
    async fn release_events(&self, ts: Timestamp) -> Result<(), Self::Error>;
}
