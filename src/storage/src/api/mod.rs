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

mod error;
mod storage;
mod storage_bucket;
mod storage_object;

pub use async_trait::async_trait;
// TODO: consider using std::stream::Stream when it is stablized.
pub use futures::stream::Stream;

pub use self::{
    error::{Error, Result},
    storage::Storage,
    storage_bucket::{StorageBucket, StorageObjectUploader},
    storage_object::StorageObject,
};
