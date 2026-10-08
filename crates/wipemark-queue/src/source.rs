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

    /// Where an engine handed out **now** would send a document — this
    /// machine, or an endpoint's origin — without building one, reading a
    /// key or loading anything; `None` when the source cannot say (nothing
    /// on duty, or a source that never sends anything away). What an item's
    /// consent is checked against before it starts (D361). Never blocks.
    fn whereto(&self) -> Option<Whereto> {
        None
    }
}

/// Where a document goes to be rewritten, as far as the person is
/// concerned: this machine, or away to an endpoint, named by its origin
/// (D361). What a surface records as the consent of an item it pushes, and
/// what an [`EngineSource`] says of the engine it would hand out.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Whereto {
    /// The document stays on this machine.
    Here,
    /// The document is sent to the endpoint at this origin.
    Away(String),
}

impl Whereto {
    /// The row's spelling. A format.
    pub(crate) fn to_value(&self) -> serde_json::Value {
        match self {
            Whereto::Here => serde_json::json!("here"),
            Whereto::Away(origin) => serde_json::json!({ "away": origin }),
        }
    }

    /// The row's spelling read back; `None` for anything else.
    pub(crate) fn of_value(value: &serde_json::Value) -> Option<Whereto> {
        match value {
            serde_json::Value::String(here) if here == "here" => Some(Whereto::Here),
            serde_json::Value::Object(object) => object
                .get("away")
                .and_then(serde_json::Value::as_str)
                .map(|origin| Whereto::Away(origin.to_owned())),
            _ => None,
        }
    }
}

/// One engine, the same for every item — what [`crate::Queue::open`] and
/// [`crate::Queue::on`] run on.
pub struct Fixed(pub Arc<dyn RewriteEngine>);

impl EngineSource for Fixed {
    fn for_item(&self) -> Result<Arc<dyn RewriteEngine>, Unavailable> {
        Ok(Arc::clone(&self.0))
    }
}
