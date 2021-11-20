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

use std::path::{Path, PathBuf};

use tokio::fs;

use super::{
    bucket::FsBucket,
    error::{Error, Result},
    object::FsObject,
};
use crate::{async_trait, Storage};

pub struct FsStorage {
    root: PathBuf,
}

impl FsStorage {
    pub async fn from(root: impl AsRef<Path>) -> Result<Self> {
        let path = root.as_ref();
        fs::DirBuilder::new()
            .recursive(true)
            .create(path.to_owned())
            .await?;
        Ok(Self {
            root: path.to_owned(),
        })
    }

    fn bucket_path(&self, name: impl Into<String>) -> PathBuf {
        let mut path = self.root.to_owned();
        path.push(name.into());
        path
    }
}

#[async_trait]
impl Storage<FsObject, FsBucket> for FsStorage {
    async fn bucket(&self, name: &str) -> Result<FsBucket> {
        let path = self.bucket_path(name);

        if fs::metadata(path.as_path()).await.is_err() {
            return Err(Error::NotFound(name.to_owned()));
        }

        Ok(FsBucket::new(path))
    }

    async fn create_bucket(&self, name: &str) -> Result<FsBucket> {
        let path = self.bucket_path(name);

        if fs::metadata(path.as_path()).await.is_ok() {
            return Err(Error::AlreadyExists(name.to_owned()));
        }

        fs::create_dir_all(path.as_path()).await?;

        Ok(FsBucket::new(path))
    }

    async fn delete_bucket(&self, name: &str) -> Result<()> {
        let path = self.bucket_path(name);

        fs::remove_dir(path.as_path()).await?;

        Ok(())
    }
}
