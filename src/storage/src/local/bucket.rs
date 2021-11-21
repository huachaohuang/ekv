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

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use tokio::{fs, io::AsyncWriteExt};

use super::{
    error::{Error, Result},
    object::LocalObject,
};
use crate::{async_trait, Bucket, ObjectUploader};

pub struct LocalBucket<'a> {
    path: Cow<'a, Path>,
}

impl<'a> LocalBucket<'a> {
    pub fn new(path: impl Into<Cow<'a, Path>>) -> Self {
        Self { path: path.into() }
    }

    fn object_path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.path.as_ref().join(name)
    }
}

#[async_trait]
impl<'a> Bucket<LocalObject<'a>> for LocalBucket<'a> {
    type ObjectUploader = LocalObjectUploader<'a>;

    async fn object(&self, name: &str) -> Result<LocalObject<'a>> {
        let path = self.object_path(name);
        if fs::metadata(path.as_path()).await.is_err() {
            return Err(Error::NotFound(name.to_owned()));
        }
        Ok(LocalObject::new(path))
    }

    async fn upload_object(&self, name: &str) -> Result<LocalObjectUploader<'a>> {
        let path = self.object_path(name);
        Ok(LocalObjectUploader::new(path))
    }

    async fn delete_object(&self, name: &str) -> Result<()> {
        let path = self.object_path(name);
        fs::remove_file(path.as_path()).await?;
        Ok(())
    }
}

pub struct LocalObjectUploader<'a> {
    path: Cow<'a, Path>,
    buf: Vec<u8>,
}

impl<'a> LocalObjectUploader<'a> {
    pub fn new(path: impl Into<Cow<'a, Path>>) -> Self {
        Self {
            path: path.into(),
            buf: vec![],
        }
    }
}

#[async_trait]
impl<'a> ObjectUploader for LocalObjectUploader<'a> {
    type Error = Error;

    async fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.buf.extend_from_slice(buf);
        Ok(())
    }

    async fn finish(self) -> Result<usize> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.path.as_ref())
            .await?;

        let v: &[u8] = &self.buf;
        f.write_all(v).await?;
        f.sync_all().await?;

        Ok(1)
    }
}
