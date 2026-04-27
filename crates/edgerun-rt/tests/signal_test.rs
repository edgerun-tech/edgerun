use edgerun_rt::SignalKind;

#[test]
fn signal_kind_variants() {
    let _ = SignalKind::Interrupt;
    let _ = SignalKind::Termination;
    let _ = SignalKind::Child;
}
