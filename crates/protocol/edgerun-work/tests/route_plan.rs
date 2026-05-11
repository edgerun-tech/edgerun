use edgerun_work::*;

#[test]
fn admission_can_prebuild_and_verify_storage_route_plan() {
    let admission = SimNode::from_seed(91, NODE_ROLE_ADMISSION);
    let relay = SimNode::from_seed(92, NODE_ROLE_RELAY);
    let storage0 = SimNode::from_seed(93, NODE_ROLE_STORAGE);
    let storage1 = SimNode::from_seed(94, NODE_ROLE_STORAGE);
    let storage2 = SimNode::from_seed(95, NODE_ROLE_STORAGE);

    let routes = vec![
        relay.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_RELAY]),
        storage0.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]),
        storage1.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]),
        storage2.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]),
    ];

    let snapshot = sign_route_snapshot(
        &admission.key,
        RouteSnapshot {
            abi_version: WORK_WIRE_ABI_VERSION,
            issued_by: admission.identity.clone(),
            sequence: 1,
            route_root: route_root_hash(&routes),
            routes,
            signature: empty_signature(),
        },
    );

    let plan = VerifiedRoutePlan::from_snapshot(snapshot).expect("verified route plan");
    let storage_routes = plan.routes_for_department(DEPARTMENT_STORAGE);
    assert_eq!(storage_routes.len(), 3);

    let assigned_nodes = [
        storage_routes[0].node.node_id,
        storage_routes[1].node.node_id,
        storage_routes[2].node.node_id,
    ];
    let file = b"admission route plan drives erasure storage".to_vec();
    let (manifest, shards) = encode_xor_2_1(&file, assigned_nodes).expect("encode");

    for shard in &shards {
        assert!(plan.route_for_node(shard.assigned_node).is_some());
    }
    verify_manifest(&manifest, &shards).expect("manifest is tied to assigned nodes");
}

#[test]
fn admission_route_plan_rejects_tampered_route_root() {
    let admission = SimNode::from_seed(101, NODE_ROLE_ADMISSION);
    let relay = SimNode::from_seed(102, NODE_ROLE_RELAY);
    let storage = SimNode::from_seed(103, NODE_ROLE_STORAGE);

    let routes = vec![
        relay.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_RELAY]),
        storage.advertise_memory_route(relay.identity.node_id, vec![DEPARTMENT_STORAGE]),
    ];

    let snapshot = sign_route_snapshot(
        &admission.key,
        RouteSnapshot {
            abi_version: WORK_WIRE_ABI_VERSION,
            issued_by: admission.identity.clone(),
            sequence: 1,
            route_root: [9u8; 32],
            routes,
            signature: empty_signature(),
        },
    );

    assert!(matches!(
        VerifiedRoutePlan::from_snapshot(snapshot),
        Err(RoutePlanError::RouteRootMismatch)
    ));
}
