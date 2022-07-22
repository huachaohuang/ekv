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

#![feature(backtrace)]

mod helper;

use std::time::Duration;

use engula_api::{server::v1::*, v1::PutRequest};
use tracing::{info, warn};

use crate::helper::{client::*, context::*, init::setup_panic_hook, runtime::block_on_current};

#[ctor::ctor]
fn init() {
    setup_panic_hook();
    tracing_subscriber::fmt::init();
}

async fn move_shard(
    c: &ClusterClient,
    shard_desc: &ShardDesc,
    dest_group_id: u64,
    src_group_id: u64,
) {
    use collect_migration_state_response::State;

    'OUTER: for _ in 0..10 {
        let src_group_epoch = c.must_group_epoch(src_group_id).await;

        // Shard migration is finished.
        if c.group_contains_shard(dest_group_id, shard_desc.id) {
            return;
        }

        let mut g = c.group(dest_group_id);
        let req = AcceptShardRequest {
            src_group_id,
            src_group_epoch,
            shard_desc: Some(shard_desc.clone()),
        };
        if let Err(e) = g.accept_shard(req).await {
            warn!(
                "accept shard {} from {src_group_id} to {dest_group_id}: {e:?}",
                shard_desc.id
            );
            continue;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
        for _ in 0..1000 {
            if let Some(leader_node_id) = c.get_group_leader_node_id(dest_group_id).await {
                if let Ok(resp) = c
                    .collect_migration_state(dest_group_id, leader_node_id)
                    .await
                {
                    if resp.state == State::None as i32 {
                        // migration is finished or aborted.
                        continue 'OUTER;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        panic!("migration task is timeout");
    }

    panic!("move shard is failed after 10 retries");
}

async fn insert(c: &ClusterClient, group_id: u64, shard_id: u64, range: std::ops::Range<u64>) {
    let mut c = c.group(group_id);
    for i in range {
        let key = format!("key-{}", i);
        let value = format!("value-{}", i);
        let req = PutRequest {
            key: key.as_bytes().to_vec(),
            value: value.as_bytes().to_vec(),
        };
        c.put(shard_id, req).await.unwrap();
    }
}

/// Migration test within groups which have only one member, shard is empty.
#[test]
fn single_replica_empty_shard_migration() {
    block_on_current(async {
        let mut ctx = TestContext::new("single-replica-empty-shard-migration");
        ctx.disable_shard_balance();
        let nodes = ctx.bootstrap_servers(2).await;
        let c = ClusterClient::new(nodes).await;
        let node_1_id = 0;
        let node_2_id = 1;
        let group_id_1 = 100000;
        let group_id_2 = 100001;
        let replica_1 = 1000000;
        let replica_2 = 2000000;
        let shard_id = 10000000;

        info!(
            "create group {} at node {} with replica {} and shard {}",
            group_id_1, node_1_id, replica_1, shard_id,
        );

        let shard_desc = ShardDesc {
            id: shard_id,
            collection_id: shard_id,
            partition: Some(shard_desc::Partition::Range(
                shard_desc::RangePartition::default(),
            )),
        };
        let replica_desc_1 = ReplicaDesc {
            id: replica_1,
            node_id: node_1_id,
            role: ReplicaRole::Voter as i32,
        };
        let group_desc_1 = GroupDesc {
            id: group_id_1,
            shards: vec![shard_desc.clone()],
            replicas: vec![replica_desc_1.clone()],
            ..Default::default()
        };
        c.create_replica(node_1_id, replica_1, group_desc_1.clone())
            .await;

        info!(
            "create group {} at node {} with replica {}",
            group_id_2, node_2_id, replica_2
        );
        let replica_desc_2 = ReplicaDesc {
            id: replica_2,
            node_id: node_2_id,
            role: ReplicaRole::Voter as i32,
        };
        let group_desc_2 = GroupDesc {
            id: group_id_2,
            shards: vec![],
            replicas: vec![replica_desc_2.clone()],
            ..Default::default()
        };
        c.create_replica(node_2_id, replica_2, group_desc_2.clone())
            .await;

        info!(
            "issue accept shard {} request to group {}",
            shard_id, group_id_2
        );

        move_shard(&c, &shard_desc, group_id_2, group_id_1).await;
    });
}

/// Migration test within groups which have only one member, shard have 1000 key values.
#[test]
fn single_replica_migration() {
    block_on_current(async {
        let mut ctx = TestContext::new("single-replica-migration");
        ctx.disable_shard_balance();
        let nodes = ctx.bootstrap_servers(2).await;
        let c = ClusterClient::new(nodes).await;
        let node_1_id = 0;
        let node_2_id = 1;
        let group_id_1 = 100000;
        let group_id_2 = 100001;
        let replica_1 = 1000000;
        let replica_2 = 2000000;
        let shard_id = 10000000;

        info!(
            "create group {} at node {} with replica {} and shard {}",
            group_id_1, node_1_id, replica_1, shard_id,
        );

        let shard_desc = ShardDesc {
            id: shard_id,
            collection_id: shard_id,
            partition: Some(shard_desc::Partition::Range(
                shard_desc::RangePartition::default(),
            )),
        };
        let replica_desc_1 = ReplicaDesc {
            id: replica_1,
            node_id: node_1_id,
            role: ReplicaRole::Voter as i32,
        };
        let group_desc_1 = GroupDesc {
            id: group_id_1,
            shards: vec![shard_desc.clone()],
            replicas: vec![replica_desc_1.clone()],
            ..Default::default()
        };
        c.create_replica(node_1_id, replica_1, group_desc_1.clone())
            .await;

        info!("insert data into group {} shard {}", group_id_1, shard_id);
        insert(&c, group_id_1, shard_id, 0..1000).await;

        info!(
            "create group {} at node {} with replica {}",
            group_id_2, node_2_id, replica_2
        );
        let replica_desc_2 = ReplicaDesc {
            id: replica_2,
            node_id: node_2_id,
            role: ReplicaRole::Voter as i32,
        };
        let group_desc_2 = GroupDesc {
            id: group_id_2,
            shards: vec![],
            replicas: vec![replica_desc_2.clone()],
            ..Default::default()
        };
        c.create_replica(node_2_id, replica_2, group_desc_2.clone())
            .await;

        info!(
            "issue accept shard {} request to group {}",
            shard_id, group_id_2
        );

        move_shard(&c, &shard_desc, group_id_2, group_id_1).await;
    });
}

async fn create_group(
    c: &ClusterClient,
    id: u64,
    replica_ids: Vec<(u64, u64)>,
    shards: Vec<ShardDesc>,
) {
    let replicas = replica_ids
        .iter()
        .cloned()
        .map(|(id, node_id)| ReplicaDesc {
            id,
            node_id,
            role: ReplicaRole::Voter as i32,
        })
        .collect();
    let group_desc = GroupDesc {
        id,
        shards,
        replicas,
        ..Default::default()
    };
    for (replica_id, node_id) in replica_ids {
        c.create_replica(node_id, replica_id, group_desc.clone())
            .await;
    }
}

/// The basic migration test.
#[test]
fn basic_migration() {
    block_on_current(async {
        let mut ctx = TestContext::new("basic-migration");
        ctx.disable_shard_balance();
        let nodes = ctx.bootstrap_servers(3).await;
        let c = ClusterClient::new(nodes).await;
        let node_1_id = 0;
        let node_2_id = 1;
        let node_3_id = 2;

        let group_id_1 = 100000;
        let group_id_2 = 100001;
        let replica_1_1 = 1000001;
        let replica_1_2 = 1000002;
        let replica_1_3 = 1000003;
        let replica_2_1 = 2000001;
        let replica_2_2 = 2000002;
        let replica_2_3 = 2000003;
        let shard_id = 10000000;

        info!("create group {} with shard {}", group_id_1, shard_id,);

        let shard_desc = ShardDesc {
            id: shard_id,
            collection_id: shard_id,
            partition: Some(shard_desc::Partition::Range(
                shard_desc::RangePartition::default(),
            )),
        };
        create_group(
            &c,
            group_id_1,
            vec![
                (replica_1_1, node_1_id),
                (replica_1_2, node_2_id),
                (replica_1_3, node_3_id),
            ],
            vec![shard_desc.clone()],
        )
        .await;

        info!("insert data into group {} shard {}", group_id_1, shard_id);
        insert(&c, group_id_1, shard_id, 0..1000).await;

        info!("create group {} ", group_id_2);
        create_group(
            &c,
            group_id_2,
            vec![
                (replica_2_1, node_1_id),
                (replica_2_2, node_2_id),
                (replica_2_3, node_3_id),
            ],
            vec![],
        )
        .await;
        info!(
            "issue accept shard {} request to group {}",
            shard_id, group_id_2
        );

        move_shard(&c, &shard_desc, group_id_2, group_id_1).await;
    });
}

#[test]
fn abort_migration() {
    block_on_current(async move {
        let mut ctx = TestContext::new("abort-migration");
        ctx.disable_all_balance();
        let nodes = ctx.bootstrap_servers(3).await;
        let c = ClusterClient::new(nodes).await;
        let node_1_id = 0;
        let node_2_id = 1;
        let node_3_id = 2;

        let group_id_1 = 100000;
        let group_id_2 = 100001;
        let replica_1_1 = 1000001;
        let replica_1_2 = 1000002;
        let replica_1_3 = 1000003;
        let replica_2_1 = 2000001;
        let replica_2_2 = 2000002;
        let replica_2_3 = 2000003;
        let shard_id = 10000000;

        info!("create group {} with shard {}", group_id_1, shard_id,);

        let shard_desc = ShardDesc {
            id: shard_id,
            collection_id: shard_id,
            partition: Some(shard_desc::Partition::Range(
                shard_desc::RangePartition::default(),
            )),
        };
        create_group(
            &c,
            group_id_1,
            vec![
                (replica_1_1, node_1_id),
                (replica_1_2, node_2_id),
                (replica_1_3, node_3_id),
            ],
            vec![shard_desc.clone()],
        )
        .await;

        info!("create group {} ", group_id_2);
        create_group(
            &c,
            group_id_2,
            vec![
                (replica_2_1, node_1_id),
                (replica_2_2, node_2_id),
                (replica_2_3, node_3_id),
            ],
            vec![],
        )
        .await;
        info!(
            "issue accept shard {} request to group {}",
            shard_id, group_id_2
        );

        let src_epoch = c.must_group_epoch(group_id_1).await;
        let mut group_client = c.group(group_id_1);
        group_client.transfer_leader(replica_1_1).await.unwrap();
        group_client
            .remove_replica(replica_1_3, node_3_id)
            .await
            .unwrap();

        let mut group_client = c.group(group_id_2);
        let req = AcceptShardRequest {
            src_group_id: group_id_1,
            src_group_epoch: src_epoch,
            shard_desc: Some(shard_desc.to_owned()),
        };

        // It will be reject by service busy?
        // Ensure issue at least one shard migartion.
        while group_client.accept_shard(req.clone()).await.is_err() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        // Ensure the formar shard migration is aborted by epoch not match.
        while group_client.accept_shard(req.clone()).await.is_err() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });
}

#[test]
fn migration_with_offline_peers() {
    block_on_current(async {
        let mut ctx = TestContext::new("basic-migration");
        ctx.disable_shard_balance();
        let nodes = ctx.bootstrap_servers(3).await;
        let c = ClusterClient::new(nodes).await;
        let node_1_id = 0;
        let node_2_id = 1;
        let node_3_id = 2;

        let group_id_1 = 100000;
        let group_id_2 = 100001;
        let replica_1_1 = 1000001;
        let replica_1_2 = 1000002;
        let replica_1_3 = 1000003;
        let replica_2_1 = 2000001;
        let replica_2_2 = 2000002;
        let replica_2_3 = 2000003;
        let shard_id = 10000000;

        info!("create group {} with shard {}", group_id_1, shard_id,);

        let shard_desc = ShardDesc {
            id: shard_id,
            collection_id: shard_id,
            partition: Some(shard_desc::Partition::Range(
                shard_desc::RangePartition::default(),
            )),
        };
        create_group(
            &c,
            group_id_1,
            vec![
                (replica_1_1, node_1_id),
                (replica_1_2, node_2_id),
                (replica_1_3, node_3_id),
            ],
            vec![shard_desc.clone()],
        )
        .await;

        info!("insert data into group {} shard {}", group_id_1, shard_id);
        insert(&c, group_id_1, shard_id, 0..1000).await;

        info!("create group {} ", group_id_2);
        create_group(
            &c,
            group_id_2,
            vec![
                (replica_2_1, node_1_id),
                (replica_2_2, node_2_id),
                (replica_2_3, node_3_id),
            ],
            vec![],
        )
        .await;
        info!(
            "issue accept shard {} request to group {}",
            shard_id, group_id_2
        );

        ctx.stop_server(2).await;

        move_shard(&c, &shard_desc, group_id_2, group_id_1).await;
    });
}
