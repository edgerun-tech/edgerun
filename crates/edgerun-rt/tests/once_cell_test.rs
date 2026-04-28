use edgerun_rt::OnceCell;
use core::panic::AssertUnwindSafe;

#[test]
fn once_cell_new() {
    let _cell: OnceCell<i32> = OnceCell::new();
}

#[test]
fn once_cell_try_insert() {
    let cell: OnceCell<i32> = OnceCell::new();
    let result = cell.try_insert(42);
    assert!(result.is_ok());
}

#[test]
fn once_cell_recover_from_init_panic() {
    let cell = OnceCell::new();

    let first = std::panic::catch_unwind(AssertUnwindSafe(|| {
        cell.get_or_init(|| panic!("bootstrap fail"));
    }));
    assert!(first.is_err());
    assert_eq!(cell.try_insert(7), Ok(&7));
    assert_eq!(cell.try_insert(8), Err((&7, 8)));
}
