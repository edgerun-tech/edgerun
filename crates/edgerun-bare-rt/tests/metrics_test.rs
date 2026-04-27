use edgerun_bare_rt::RuntimeMetrics;

#[test]
fn runtime_metrics_new() {
    let metrics = RuntimeMetrics::new();
    let _ = metrics;
}
