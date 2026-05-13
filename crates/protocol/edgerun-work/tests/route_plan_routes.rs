use edgerun_crypto::Ed25519SigningKey;
use edgerun_work::*;

fn signed_memory_route(
    node: &SimNode,
    department: u16,
    status: u16,
    valid_until_unix_ms: u64,
) -> RouteAdvertisement {
    let mut route = node.advertise_memory_route(node.identity.node_id, vec![department]);
    route.status = status;
    route.valid_until_unix_ms = valid_until_unix_ms;
    sign_route_advertisement(&node.key, route)
}

fn plan_from_routes(routes: Vec<RouteAdvertisement>) -> VerifiedRoutePlan {
    let key = Ed25519SigningKey::from_bytes(&[77u8; 32]);
    let route_root = route_root_hash(&routes);
    let snapshot = sign_route_snapshot(
        &key,
        RouteSnapshot {
            abi_version: WORK_WIRE_ABI_VERSION,
            issued_by: node_identity_from_key(&key, NODE_ROLE_RELAY),
            sequence: 1,
            routes,
            route_root,
            signature: empty_signature(),
        },
    );
    VerifiedRoutePlan::from_snapshot(snapshot).expect("verified route snapshot")
}

#[test]
fn route_plan_filters_expired_department_routes() {
    let expired = SimNode::from_seed(78, NODE_ROLE_MESSAGE);
    let fresh = SimNode::from_seed(79, NODE_ROLE_MESSAGE);
    let plan = plan_from_routes(vec![
        signed_memory_route(&expired, DEPARTMENT_MESSAGE, ROUTE_STATUS_AVAILABLE, 10),
        signed_memory_route(&fresh, DEPARTMENT_MESSAGE, ROUTE_STATUS_AVAILABLE, 100),
    ]);

    let selected = plan
        .first_route_for_department_at(DEPARTMENT_MESSAGE, 50)
        .expect("fresh route selected");

    assert_eq!(selected.node.node_id, fresh.identity.node_id);
    assert_eq!(
        plan.routes_for_department_at(DEPARTMENT_MESSAGE, 50).len(),
        1
    );
    assert_eq!(plan.route_for_node_at(expired.identity.node_id, 50), None,);
}

#[test]
fn route_plan_filters_unavailable_department_routes() {
    let unavailable = SimNode::from_seed(80, NODE_ROLE_MESSAGE);
    let fresh = SimNode::from_seed(81, NODE_ROLE_MESSAGE);
    let plan = plan_from_routes(vec![
        signed_memory_route(
            &unavailable,
            DEPARTMENT_MESSAGE,
            ROUTE_STATUS_UNAVAILABLE,
            100,
        ),
        signed_memory_route(&fresh, DEPARTMENT_MESSAGE, ROUTE_STATUS_AVAILABLE, 100),
    ]);

    let selected = plan
        .first_route_for_department_at(DEPARTMENT_MESSAGE, 50)
        .expect("available route selected");

    assert_eq!(selected.node.node_id, fresh.identity.node_id);
    assert_eq!(
        plan.routes_for_department_at(DEPARTMENT_MESSAGE, 50).len(),
        1
    );
    assert_eq!(
        plan.route_for_node_at(unavailable.identity.node_id, 50),
        None,
    );
}

#[test]
fn route_plan_require_department_route_reports_no_route_when_all_stale() {
    let expired = SimNode::from_seed(82, NODE_ROLE_MESSAGE);
    let plan = plan_from_routes(vec![signed_memory_route(
        &expired,
        DEPARTMENT_STORAGE,
        ROUTE_STATUS_AVAILABLE,
        10,
    )]);

    assert_eq!(
        plan.require_department_route_at(DEPARTMENT_STORAGE, 50),
        Err(RoutePlanError::NoRoute),
    );
}
