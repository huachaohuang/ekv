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
mod applier;
mod facade;
mod fsm;
mod metrics;
mod node;
pub mod snap;
mod storage;
mod transport;
mod worker;

use std::{path::Path, sync::Arc};

use engula_api::server::v1::*;
use raft::prelude::{
    ConfChangeSingle, ConfChangeTransition, ConfChangeType, ConfChangeV2, ConfState,
};
use serde::{Deserialize, Serialize};

use self::worker::RaftWorker;
pub use self::{
    facade::RaftNodeFacade,
    fsm::{ApplyEntry, SnapshotBuilder, StateMachine},
    snap::SnapManager,
    storage::write_initial_state,
    transport::{retrive_snapshot, AddressResolver, TransportManager},
    worker::{RaftGroupState, StateObserver},
};
use crate::{
    runtime::{sync::WaitGroup, Executor, TaskPriority},
    Provider, Result,
};

#[derive(Clone, Debug, Default)]
pub struct RaftTestingKnobs {
    pub force_new_peer_receiving_snapshot: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RaftConfig {
    /// The intervals of tick, in millis.
    ///
    /// Default: 500ms.
    pub tick_interval_ms: u64,

    /// The size of inflights requests.
    ///
    /// Default: 102400
    pub max_inflight_requests: usize,

    /// Before a follower begin election, it must wait a randomly election ticks and does not
    /// receives any messages from leader.
    ///
    /// Default: 3.
    pub election_tick: usize,

    /// Limit the entries batched in an append message(in size). 0 means one entry per message.
    ///
    /// Default: 64KB
    pub max_size_per_msg: u64,

    /// Limit the number of inflights messages which send to one peer.
    ///
    /// Default: 10K
    pub max_inflight_msgs: usize,

    #[serde(skip)]
    pub testing_knobs: RaftTestingKnobs,
}

/// `ReadPolicy` is used to control `RaftNodeFacade::read` behavior.
#[derive(Debug, Clone, Copy)]
pub enum ReadPolicy {
    /// Do nothing
    Relaxed,
    /// Wait until all former committed entries be applied.
    LeaseRead,
    /// Like `ReadPolicy::LeaseRead`, but require exchange heartbeat with majority members before
    /// waiting.
    ReadIndex,
}

#[derive(Clone)]
pub struct RaftManager {
    pub cfg: RaftConfig,
    provider: Arc<Provider>,
    engine: Arc<raft_engine::Engine>,
    transport_mgr: TransportManager,
    snap_mgr: SnapManager,
}

impl RaftManager {
    pub(crate) fn open(
        cfg: RaftConfig,
        provider: Arc<Provider>,
        transport_mgr: TransportManager,
    ) -> Result<Self> {
        use raft_engine::{Config, Engine};
        let engine_dir = provider.log_path.join("engine");
        let snap_dir = provider.log_path.join("snap");
        create_dir_all_if_not_exists(&engine_dir)?;
        create_dir_all_if_not_exists(&snap_dir)?;
        let engine_cfg = Config {
            dir: engine_dir.to_str().unwrap().to_owned(),
            ..Default::default()
        };
        let engine = Arc::new(Engine::open(engine_cfg)?);
        let snap_mgr = SnapManager::recovery(&provider.executor, snap_dir)?;
        Ok(RaftManager {
            cfg,
            provider,
            engine,
            transport_mgr,
            snap_mgr,
        })
    }

    #[inline]
    pub fn engine(&self) -> &raft_engine::Engine {
        &self.engine
    }

    #[inline]
    pub fn snapshot_manager(&self) -> &SnapManager {
        &self.snap_mgr
    }

    #[inline]
    pub async fn list_groups(&self) -> Vec<u64> {
        self.engine.raft_groups()
    }

    #[inline]
    pub fn executor(&self) -> &Executor {
        &self.provider.executor
    }

    pub async fn start_raft_group<M: 'static + StateMachine>(
        &self,
        group_id: u64,
        replica_id: u64,
        node_id: u64,
        state_machine: M,
        observer: Box<dyn StateObserver>,
        wait_group: WaitGroup,
    ) -> Result<RaftNodeFacade> {
        let worker =
            RaftWorker::open(group_id, replica_id, node_id, state_machine, self, observer).await?;
        let facade = RaftNodeFacade::open(worker.request_sender());

        let tick_interval_ms = self.cfg.tick_interval_ms;
        let tag = &group_id.to_le_bytes();
        self.provider
            .executor
            .spawn(Some(tag), TaskPriority::High, async move {
                // TODO(walter) handle result.
                worker.run(tick_interval_ms).await.unwrap();
                drop(wait_group);
            });
        Ok(facade)
    }
}

impl Default for RaftConfig {
    fn default() -> Self {
        RaftConfig {
            tick_interval_ms: 500,
            max_inflight_requests: 102400,
            election_tick: 3,
            max_size_per_msg: 64 * 1024 * 1024,
            max_inflight_msgs: 10 * 1000,
            testing_knobs: RaftTestingKnobs::default(),
        }
    }
}

fn encode_to_conf_change(change_replicas: ChangeReplicas) -> ConfChangeV2 {
    use prost::Message;

    let mut conf_changes = vec![];
    for c in &change_replicas.changes {
        let change_type = match ChangeReplicaType::from_i32(c.change_type) {
            Some(ChangeReplicaType::Add) => ConfChangeType::AddNode,
            Some(ChangeReplicaType::Remove) => ConfChangeType::RemoveNode,
            Some(ChangeReplicaType::AddLearner) => ConfChangeType::AddLearnerNode,
            None => panic!("such change replica operation isn't supported"),
        };
        conf_changes.push(ConfChangeSingle {
            change_type: change_type.into(),
            node_id: c.replica_id,
        });
    }

    ConfChangeV2 {
        transition: ConfChangeTransition::Auto.into(),
        context: change_replicas.encode_to_vec(),
        changes: conf_changes,
    }
}

fn decode_from_conf_change(conf_change: &ConfChangeV2) -> ChangeReplicas {
    use prost::Message;

    ChangeReplicas::decode(&*conf_change.context)
        .expect("ChangeReplicas is saved in ConfChangeV2::context")
}

pub fn conf_state_from_group_descriptor(desc: &GroupDesc) -> ConfState {
    let mut cs = ConfState::default();
    let mut in_joint = false;
    for replica in desc.replicas.iter() {
        match ReplicaRole::from_i32(replica.role).unwrap_or(ReplicaRole::Voter) {
            ReplicaRole::Voter => {
                cs.voters.push(replica.id);
            }
            ReplicaRole::Learner => {
                cs.learners.push(replica.id);
                cs.learners_next.push(replica.id);
            }
            ReplicaRole::IncomingVoter => {
                in_joint = true;
                cs.voters.push(replica.id);
            }
            ReplicaRole::DemotingVoter => {
                in_joint = true;
                cs.voters_outgoing.push(replica.id);
                cs.learners_next.push(replica.id);
            }
        }
    }
    if !in_joint {
        cs.voters_outgoing.clear();
    }
    cs
}

fn create_dir_all_if_not_exists<P: AsRef<Path>>(dir: &P) -> Result<()> {
    use std::io::ErrorKind;
    match std::fs::create_dir_all(dir.as_ref()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err.into()),
    }
}
