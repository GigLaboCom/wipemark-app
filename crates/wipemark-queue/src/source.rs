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
    /// The engine for the item starting now, and where **that engine**
    /// sends a document — read together, from the one place the engine
    /// comes from, so what an item's consent is checked against is the
    /// engine's own destination and never a record of where the duty was
    /// meant to be by then (D361, D370).
    fn for_item(&self) -> Result<Handed, Unavailable>;

    /// Where the engine [`for_item`](Self::for_item) would hand out now
    /// sends a document, said **without** building it or reading a key —
    /// so an item that would only be asked about costs no credential-store
    /// read, which on macOS can put a keychain prompt on screen (D396).
    /// `None` when the source cannot say without building; then only the
    /// engine handed out is checked.
    fn whereto(&self) -> Option<Whereto> {
        None
    }

    /// Whether the source is about to hand out another engine — a swap it
    /// deferred while a job ran, landing as the job ends. An item does not
    /// start on the engine that is on its way out: the queue waits, and is
    /// told by [`crate::Queue::engine_changed`] when it may look again
    /// (D395). Never `true` for longer than the swap takes.
    fn settling(&self) -> bool {
        false
    }
}

/// An engine handed out for one item, with where it sends a document.
pub struct Handed {
    pub engine: Arc<dyn RewriteEngine>,
    /// Where this engine sends a document: this machine, or an endpoint's
    /// origin. `None` when the source cannot say — one engine a caller
    /// chose ([`Fixed`]) — and then nothing is asked about: whoever built
    /// the queue on it chose where a document goes.
    pub whereto: Option<Whereto>,
}

impl Handed {
    /// An engine whose destination the source does not say.
    pub fn unsaid(engine: Arc<dyn RewriteEngine>) -> Handed {
        Handed {
            engine,
            whereto: None,
        }
    }
}

impl std::fmt::Debug for Handed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Handed")
            .field("whereto", &self.whereto)
            .finish_non_exhaustive()
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
    fn for_item(&self) -> Result<Handed, Unavailable> {
        Ok(Handed::unsaid(Arc::clone(&self.0)))
    }
}
