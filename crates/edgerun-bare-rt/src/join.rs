//! Join utilities for running multiple futures concurrently

use core::future::Future;

pub async fn join2<A, B>(a: A, b: B) -> (A::Output, B::Output)
where
    A: Future,
    B: Future,
{
    (a.await, b.await)
}

pub async fn join3<A, B, C>(a: A, b: B, c: C) -> (A::Output, B::Output, C::Output)
where
    A: Future,
    B: Future,
    C: Future,
{
    (a.await, b.await, c.await)
}

pub async fn join4<A, B, C, D>(a: A, b: B, c: C, d: D) -> (A::Output, B::Output, C::Output, D::Output)
where
    A: Future,
    B: Future,
    C: Future,
    D: Future,
{
    (a.await, b.await, c.await, d.await)
}

pub async fn join5<A, B, C, D, E>(a: A, b: B, c: C, d: D, e: E) -> (A::Output, B::Output, C::Output, D::Output, E::Output)
where
    A: Future,
    B: Future,
    C: Future,
    D: Future,
    E: Future,
{
    (a.await, b.await, c.await, d.await, e.await)
}