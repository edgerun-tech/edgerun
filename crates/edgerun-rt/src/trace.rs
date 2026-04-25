//! Lightweight async-aware tracing without external dependencies.
//!
//! Uses a thread-local span stack so workers can emit enter/exit events
//! tied to the currently executing task. Instrumentation is manual —
//! call `span!` to create, `.enter()` to begin, and drop the guard to end.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

// ===========================================================================
// Global span ID counter
// ===========================================================================

static NEXT_SPAN_ID: AtomicU64 = AtomicU64::new(1);

fn next_span_id() -> u64 {
    NEXT_SPAN_ID.fetch_add(1, Ordering::Relaxed)
}

// ===========================================================================
// Span
// ===========================================================================

/// A named tracing span with an optional parent.
///
/// Spans form a tree via the `parent_id` field. On creation, if no parent
/// is specified, the current span on the thread-local stack is used.
pub struct Span {
    id: u64,
    name: &'static str,
    parent_id: Option<u64>,
}

impl Span {
    /// Create a new span with an explicit parent.
    pub fn with_parent(name: &'static str, parent_id: u64) -> Self {
        Self {
            id: next_span_id(),
            name,
            parent_id: Some(parent_id),
        }
    }

    /// Create a new span, auto-detecting parent from the thread-local stack.
    pub fn new(name: &'static str) -> Self {
        let parent_id = with_current(|s| s.id);
        Self {
            id: next_span_id(),
            name,
            parent_id,
        }
    }

    /// Enter the span, pushing it onto the thread-local stack.
    /// Returns a guard that exits the span when dropped.
    pub fn enter(&self) -> EnterGuard<'_> {
        push_span(self);
        edgerun_log::trace!("[span:{}] enter {}", self.id, self.name);
        EnterGuard { span: self }
    }

    /// The unique ID of this span.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The name of this span.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

// ===========================================================================
// EnterGuard
// ===========================================================================

/// RAII guard — exits the span when dropped.
pub struct EnterGuard<'a> {
    span: &'a Span,
}

impl Drop for EnterGuard<'_> {
    fn drop(&mut self) {
        pop_span();
        edgerun_log::trace!("[span:{}] exit {}", self.span.id, self.span.name);
    }
}

// ===========================================================================
// Thread-local span stack
// ===========================================================================

struct SpanStack {
    stack: Vec<u64>,
}

thread_local! {
    static SPAN_STACK: RefCell<SpanStack> = const {
        RefCell::new(SpanStack { stack: Vec::new() })
    };
}

fn push_span(span: &Span) {
    SPAN_STACK.with(|s| s.borrow_mut().stack.push(span.id));
}

fn pop_span() {
    SPAN_STACK.with(|s| s.borrow_mut().stack.pop());
}

fn with_current<R, F: FnOnce(&SpanEntry) -> R>(f: F) -> Option<R> {
    SPAN_STACK.with(|s| {
        let guard = s.borrow();
        guard.stack.last().copied().map(|id| f(&SpanEntry { id }))
    })
}

struct SpanEntry {
    id: u64,
}

// ===========================================================================
// Task span registry — maps task IDs to root span IDs
// ===========================================================================

use crate::sync::Mutex;

pub(crate) struct TaskSpanMap {
    map: Mutex<std::collections::HashMap<usize, u64>>,
}

impl TaskSpanMap {
    fn new() -> Self {
        Self {
            map: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn insert(&self, task_id: usize, span_id: u64) {
        self.map.lock().insert(task_id, span_id);
    }

    fn remove(&self, task_id: usize) -> Option<u64> {
        self.map.lock().remove(&task_id)
    }
}

/// Global registry of task → root span ID.
pub(crate) static TASK_SPANS: std::sync::LazyLock<TaskSpanMap> =
    std::sync::LazyLock::new(TaskSpanMap::new);

// ===========================================================================
// Task lifecycle helpers
// ===========================================================================

/// Called when a task is spawned. Creates the root span for this task.
pub(crate) fn task_spawned(task_id: usize, name: &'static str) -> Span {
    let span = Span::new(name);
    TASK_SPANS.insert(task_id, span.id());
    span
}

/// Called when a task completes. Cleans up the root span.
pub(crate) fn task_finished(task_id: usize) {
    TASK_SPANS.remove(task_id);
}

/// Called when a task is aborted.
pub(crate) fn task_aborted(task_id: usize) {
    TASK_SPANS.remove(task_id);
}

// ===========================================================================
// Convenience macro
// ===========================================================================

/// Create and enter a span in one expression.
///
/// ```ignore
/// span!("my_operation", {
///     // code runs inside the span
/// });
/// ```
#[macro_export]
macro_rules! span {
    ($name:expr, $body:expr) => {{
        let _guard = $crate::Span::new($name).enter();
        $body
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_creates_unique_ids() {
        let a = Span::new("a");
        let b = Span::new("b");
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn span_enter_exit_stack() {
        let outer = Span::new("outer");
        let _g1 = outer.enter();

        let id1 = with_current(|s| s.id).unwrap();
        assert_eq!(id1, outer.id());

        {
            let inner = Span::new("inner");
            let _g2 = inner.enter();

            let id2 = with_current(|s| s.id).unwrap();
            assert_eq!(id2, inner.id());
            assert!(inner.parent_id == Some(outer.id()));
        }

        let id3 = with_current(|s| s.id).unwrap();
        assert_eq!(id3, outer.id());
    }

    #[test]
    fn span_parent_detection() {
        let a = Span::new("a");
        let _ga = a.enter();

        let b = Span::new("b");
        assert_eq!(b.parent_id, Some(a.id()));

        let _gb = b.enter();
        let c = Span::new("c");
        assert_eq!(c.parent_id, Some(b.id()));
    }

    #[test]
    fn span_with_explicit_parent() {
        let root = Span::new("root");
        let child = Span::with_parent("child", root.id());
        assert_eq!(child.parent_id, Some(root.id()));
    }
}
