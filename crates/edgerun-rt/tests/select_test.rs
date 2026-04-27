use core::future::Future;
use edgerun_rt::{select_2, select_3, select_4};

#[test]
fn select2_compiles() {
    fn _test<F, O>(f1: F, f2: F) -> impl Future<Output = O>
    where
        F: Future<Output = O> + Unpin,
        O: Unpin,
    {
        select_2(f1, f2)
    }
}

#[test]
fn select3_compiles() {
    fn _test<F, O>(f1: F, f2: F, f3: F) -> impl Future<Output = O>
    where
        F: Future<Output = O> + Unpin,
        O: Unpin,
    {
        select_3(f1, f2, f3)
    }
}

#[test]
fn select4_compiles() {
    fn _test<F, O>(f1: F, f2: F, f3: F, f4: F) -> impl Future<Output = O>
    where
        F: Future<Output = O> + Unpin,
        O: Unpin,
    {
        select_4(f1, f2, f3, f4)
    }
}
