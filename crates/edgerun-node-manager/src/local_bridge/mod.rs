// SPDX-License-Identifier: Apache-2.0
pub(crate) mod cloudflare;
pub(crate) mod cloudflare_handlers;
pub(crate) mod docker_handlers;
pub(crate) mod docker_local;
pub(crate) mod github_workflow_handlers;
pub(crate) mod github_workflows;

pub(crate) use cloudflare_handlers::{
    handle_local_cloudflare_access_apps, handle_local_cloudflare_dns_records,
    handle_local_cloudflare_dns_upsert, handle_local_cloudflare_pages,
    handle_local_cloudflare_tunnels, handle_local_cloudflare_verify,
    handle_local_cloudflare_workers, handle_local_cloudflare_zones,
};
pub(crate) use docker_handlers::{handle_local_docker_container_state, handle_local_docker_summary};
pub(crate) use github_workflow_handlers::{
    handle_local_github_workflow_runner_run, handle_local_github_workflow_runner_runs,
    handle_local_github_workflow_runs,
};
