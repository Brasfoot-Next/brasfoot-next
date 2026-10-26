//! Reader of Kryo 4 streams as the original game writes them (`sav/*.s22`).
//!
//! A `.s22` holds two roots, written by `c.a` with one `writeClassAndObject` each: `c.a.SR`
//! (`best.f`, the whole game state) and `c.a.afQ` (`d.ay`). The reader turns them into a
//! [`Graph`] using the wire schema generated from the game jar; it never runs game code.

mod canonical;
mod graph;
mod input;
mod reader;
mod schema;

use std::fmt;

pub use canonical::write_canonical;
pub use graph::{Calendar, ClassId, Graph, GraphClass, Node, NodeData, NodeId, StrId, Value};
pub use schema::{game_schema, ClassSchema, FieldSchema, Schema};

#[derive(Debug, Clone)]
pub struct KryoError {
    pub message: String,
    /// Byte offset in the stream.
    pub position: usize,
}

impl fmt::Display for KryoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "kryo: {} (at byte {})", self.message, self.position)
    }
}

impl std::error::Error for KryoError {}

/// Stack for the reader and for anything else that walks a game graph recursively: Kryo nests
/// objects as deep as the game's references go.
pub const DEEP_STACK: usize = 1 << 30;

/// Runs `f` on a thread with [`DEEP_STACK`] bytes of stack.
pub fn with_deep_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .name("deep-stack".into())
        .stack_size(DEEP_STACK)
        .spawn(f)
        .expect("spawn deep-stack thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// Reads every root of a Kryo stream written by the game.
pub fn read_graph(bytes: Vec<u8>) -> Result<Graph, KryoError> {
    with_deep_stack(move || reader::Reader::new(&bytes, game_schema()).read_all())
}
