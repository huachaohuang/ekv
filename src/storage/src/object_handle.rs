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

use async_trait::async_trait;
use bytes::Bytes;

use crate::StorageResult;

#[async_trait]
pub trait ObjectWriter {
    async fn write(&mut self, data: Bytes);

    async fn finish(&mut self) -> StorageResult<()>;
}

#[async_trait]
pub trait ObjectReader {
    async fn read_at(&self, offset: i32, size: i32) -> StorageResult<Vec<u8>>;
}
