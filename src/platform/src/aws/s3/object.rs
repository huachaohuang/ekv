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

use aws_sdk_s3::Client;
use bytes::Buf;
use storage::{async_trait, Error, Object, Result};

pub(crate) struct S3Object {
    client: Client,
    bucket_name: String,
    key: String,
}

impl S3Object {
    pub(crate) fn new(client: Client, bucket_name: String, key: String) -> Self {
        Self {
            client,
            bucket_name,
            key,
        }
    }
}

#[async_trait]
impl Object for S3Object {
    async fn size(&self) -> Result<usize> {
        self.client
            .head_object()
            .bucket(self.bucket_name.to_owned())
            .key(self.key.to_owned())
            .send()
            .await
            .map(|output| output.content_length as usize)
            .map_err(|e| {
                Error::AwsSDK(format!(
                    "get object size for '{}' fail, {}",
                    self.key.to_owned(),
                    e.to_string(),
                ))
            })
    }

    async fn read_at(&self, buf: &mut [u8], offset: usize) -> Result<usize> {
        let size = buf.len();
        let range = format!("bytes={}-{}", offset, offset + size - 1);
        let result = self
            .client
            .get_object()
            .bucket(self.bucket_name.to_owned())
            .key(self.key.to_owned())
            .range(range)
            .send()
            .await;

        match result {
            Ok(output) => match output.body.collect().await {
                Ok(mut bytes) => {
                    bytes.copy_to_slice(buf);
                    Ok(buf.len())
                }
                Err(e) => Err(Error::AwsSDK(format!(
                    "read object fail, {}",
                    e.to_string()
                ))),
            },
            Err(e) => Err(Error::AwsSDK(format!(
                "read object fail, {}",
                e.to_string()
            ))),
        }
    }
}
