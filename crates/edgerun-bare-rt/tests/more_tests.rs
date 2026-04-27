use edgerun_bare_rt::TaskMetrics;

#[test]
fn task_metrics_new() {
    let metrics = TaskMetrics::new("test");
    let _ = metrics;
}
