use edgerun_bare_rt::OnceCell;

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
