//! Where the queue's engine comes from: asked when each item starts.

use std::sync::Arc;

use wipemark_engine::{RewriteEngine, Unavailable};

/// What the queue asks for an engine when an item starts (R1, E4-6b).
///
/// **Blocking**, and called on the queue's own thread: the application's
/// answer may read a key from the credential store, which blocks, and the
/// queue's thread is the one place in a surface where waiting for it costs
/// nothing anybody can see. The engine handed out is held by the item's
/// job for its whole length and dropped when the job ends — which is what
/// lets an implementation count the item busy until then (the application's
/// `EngineHandle::for_job`).
///
/// An `Err` is not a failure of the item: the queue holds with the reason
/// and waits for [`crate::Queue::engine_changed`].
pub trait EngineSource: Send + Sync {
    fn for_item(&self) -> Result<Arc<dyn RewriteEngine>, Unavailable>;
}

/// One engine, the same for every item — what [`crate::Queue::open`] and
/// [`crate::Queue::on`] run on.
pub struct Fixed(pub Arc<dyn RewriteEngine>);

impl EngineSource for Fixed {
    fn for_item(&self) -> Result<Arc<dyn RewriteEngine>, Unavailable> {
        Ok(Arc::clone(&self.0))
    }
}
