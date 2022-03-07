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

use crate::{BucketIter, Result};

pub struct Store {}

impl Store {
    pub async fn tenant(&self, _name: &str) -> Result<Tenant> {
        todo!();
    }
}

pub struct Tenant {}

impl Tenant {
    pub async fn bucket(&self, _name: &str) -> Result<Bucket> {
        todo!();
    }
}

pub struct Bucket {}

impl Bucket {
    pub async fn get(&self, _id: &[u8]) -> Result<Option<Vec<u8>>> {
        todo!();
    }

    pub fn iter(&self) -> BucketIter {
        BucketIter {}
    }
}
