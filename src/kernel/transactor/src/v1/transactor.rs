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

use engula_apis::v1::*;
use engula_supervisor::v1::Supervisor;

use super::Result;

#[derive(Clone)]
pub struct Transactor {
    supervisor: Supervisor,
}

impl Default for Transactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Transactor {
    pub fn new() -> Self {
        let supervisor = Supervisor::new();
        Self { supervisor }
    }

    pub async fn batch(&self, mut batch_req: BatchRequest) -> Result<BatchResponse> {
        let mut batch_res = BatchResponse::default();
        let databases = std::mem::take(&mut batch_req.databases);
        if !databases.is_empty() {
            todo!();
        }
        let universes = std::mem::take(&mut batch_req.universes);
        if !universes.is_empty() {
            let req = engula_supervisor::v1::apis::v1::BatchRequest { universes };
            let mut res = self.supervisor.batch(req).await?;
            batch_res.universes = std::mem::take(&mut res.universes);
        }
        Ok(batch_res)
    }
}
