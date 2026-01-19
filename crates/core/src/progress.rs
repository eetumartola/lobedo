use std::cell::RefCell;
use std::sync::Arc;

use crate::graph::NodeId;

#[derive(Clone, Debug)]
pub enum ProgressEvent {
    Start { node: NodeId },
    Advance { node: NodeId, fraction: f32 },
    Finish { node: NodeId },
}

pub type ProgressSink = Arc<dyn Fn(ProgressEvent) + Send + Sync>;

#[derive(Default)]
struct ProgressContext {
    sink: Option<ProgressSink>,
    node: Option<NodeId>,
}

thread_local! {
    static CONTEXT: RefCell<ProgressContext> = RefCell::new(ProgressContext::default());
}

pub struct ProgressGuard {
    prev: ProgressContext,
}

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        let prev = std::mem::take(&mut self.prev);
        CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = prev;
        });
    }
}

pub fn set_progress_context(node: NodeId, sink: Option<ProgressSink>) -> ProgressGuard {
    let prev = CONTEXT.with(|ctx| std::mem::replace(&mut *ctx.borrow_mut(), ProgressContext { sink, node: Some(node) }));
    ProgressGuard { prev }
}

pub fn report_progress(fraction: f32) {
    let fraction = fraction.clamp(0.0, 1.0);
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        if let (Some(node), Some(sink)) = (ctx.node, ctx.sink.as_ref()) {
            (sink)(ProgressEvent::Advance { node, fraction });
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn current_progress_context() -> Option<(NodeId, ProgressSink)> {
    CONTEXT.with(|ctx| {
        let ctx = ctx.borrow();
        match (ctx.node, ctx.sink.as_ref()) {
            (Some(node), Some(sink)) => Some((node, Arc::clone(sink))),
            _ => None,
        }
    })
}
