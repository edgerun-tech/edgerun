use anyhow::{Context, Ok, Result};
use oci_spec::runtime::{ProcessBuilder, Spec, SpecBuilder};
use test_framework::{Test, TestGroup, TestResult, test_result};

use crate::utils::support::random_range_i32;
use crate::utils::test_inside_container;
use crate::utils::test_utils::CreateOptions;

fn generate_random_number() -> i32 {
    random_range_i32(300, 700)
}

fn create_spec() -> Result<Spec> {
    let spec = SpecBuilder::default()
        .process(
            ProcessBuilder::default()
                .args(vec![
                    "runtimetest".to_string(),
                    "process_oom_score_adj".to_string(),
                ])
                .oom_score_adj(generate_random_number())
                .build()
                .expect("error in creating process config"),
        )
        .build()
        .context("failed to build spec")?;

    Ok(spec)
}

fn process_oom_score_adj_test() -> TestResult {
    let spec = test_result!(create_spec());
    test_inside_container(&spec, &CreateOptions::default(), &|_| Ok(()))
}

pub fn get_process_oom_score_adj_test() -> TestGroup {
    let mut process_oom_score_adj_test_group = TestGroup::new("process_oom_score_adj");

    let test = Test::new(
        "process_oom_score_adj",
        Box::new(process_oom_score_adj_test),
    );
    process_oom_score_adj_test_group.add(vec![Box::new(test)]);

    process_oom_score_adj_test_group
}
