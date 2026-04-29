use edgerun_rt::RingBuffer;

#[test]
fn ring_buffer_push_pop_slice_wrap_around() {
    let mut ring = RingBuffer::new(8);

    assert_eq!(ring.push_slice(&[1, 2, 3, 4, 5, 6]), 6);

    let mut first = [0u8; 3];
    assert_eq!(ring.pop_slice(&mut first), 3);
    assert_eq!(&first, &[1, 2, 3]);

    assert_eq!(ring.push_slice(&[7, 8, 9]), 3);

    let mut second = [0u8; 6];
    assert_eq!(ring.pop_slice(&mut second), 6);
    assert_eq!(&second, &[4, 5, 6, 7, 8, 9]);
}

#[test]
fn ring_buffer_small_capacity_saturates() {
    let mut ring = RingBuffer::new(1);

    assert_eq!(ring.push(1), true);
    assert_eq!(ring.push(2), false);
    assert_eq!(ring.push_slice(&[3, 4]), 0);

    assert_eq!(ring.pop(), Some(1));
    assert_eq!(ring.push_slice(&[2]), 1);
    let mut buf = [0u8; 2];
    assert_eq!(ring.pop_slice(&mut buf), 1);
    assert_eq!(&buf[0], &2);
}
