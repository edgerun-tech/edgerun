use edgerun_bare_rt::broadcast_channel;

#[test]
fn broadcast_channel_created() {
    let (sender, receiver) = broadcast_channel(42i32);
    let _ = (sender, receiver);
}
