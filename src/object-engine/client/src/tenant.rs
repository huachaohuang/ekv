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

use object_engine_master::{proto::*, FileTenant};

use crate::{Bucket, BulkLoad, Master, Result};

#[derive(Clone)]
pub struct Tenant {
    name: String,
    master: Master,
    file_tenant: FileTenant,
}

impl Tenant {
    pub(crate) fn new(name: String, master: Master, file_tenant: FileTenant) -> Self {
        Self {
            name,
            master,
            file_tenant,
        }
    }

    pub async fn desc(&self) -> Result<TenantDesc> {
        self.master.get_tenant(self.name.clone()).await
    }

    pub fn bucket(&self, name: &str) -> Bucket {
        Bucket::new(name.to_owned(), self.name.clone(), self.master.clone())
    }

    pub async fn create_bucket(&self, name: &str) -> Result<BucketDesc> {
        let desc = BucketDesc {
            name: name.to_owned(),
            ..Default::default()
        };
        self.master.create_bucket(self.name.clone(), desc).await
    }

    pub async fn begin_bulkload(&self) -> Result<BulkLoad> {
        let token = self.master.begin_bulkload(self.name.clone()).await?;
        Ok(BulkLoad::new(
            token,
            self.name.clone(),
            self.master.clone(),
            self.file_tenant.clone(),
        ))
    }
}
