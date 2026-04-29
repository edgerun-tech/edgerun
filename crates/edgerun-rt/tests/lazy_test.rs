use core::panic::AssertUnwindSafe;
use edgerun_rt::LazyStatic;

#[test]
fn lazy_static_recovers_after_init_panic() {
    let cell = LazyStatic::new();

    let first = std::panic::catch_unwind(AssertUnwindSafe(|| {
        cell.get(|| panic!("init fail"));
    }));
    assert!(first.is_err());

    assert_eq!(*cell.get(|| 99), 99);
}
