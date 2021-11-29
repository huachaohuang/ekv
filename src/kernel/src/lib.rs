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

mod engine;
mod error;
mod kernel;
mod local;
mod update;
mod version;

pub use async_trait::async_trait;

pub type ResultStream<T> = Box<dyn futures::Stream<Item = Result<T>> + Unpin>;

pub use self::{
    engine::{Engine, EngineUpdate},
    error::{Error, Result},
    kernel::Kernel,
    local::{LocalEngine, LocalKernel},
    update::UpdateAction,
    version::{Sequence, Version, VersionUpdate},
};
