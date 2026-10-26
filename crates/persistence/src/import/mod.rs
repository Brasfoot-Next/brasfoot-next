//! Import of an original save (`.s22`) into the model: a one-way migration, after which the
//! career lives in the native format.

mod context;
mod generated;

use brasfoot_model::world::World;

use crate::kryo::{self, Graph, KryoError, Value};
use context::{Arena, Context};
use generated::*;

#[derive(Debug)]
pub enum ImportError {
    Kryo(KryoError),
    /// The stream does not hold the game's two roots.
    NotASave(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Kryo(e) => write!(f, "import: {e}"),
            ImportError::NotASave(why) => write!(f, "import: not a Brasfoot save ({why})"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<KryoError> for ImportError {
    fn from(e: KryoError) -> Self {
        ImportError::Kryo(e)
    }
}

/// Reads a `.s22` and converts it.
pub fn import_s22(bytes: Vec<u8>) -> Result<World, ImportError> {
    let graph = kryo::read_graph(bytes)?;
    import_graph(&graph)
}

/// Converts the graph of a `.s22` (roots `best.f` and `best.ay`). A graph that breaks the
/// game's invariants (an object where the game keeps a number, say) is reported, not trusted.
pub fn import_graph(graph: &Graph) -> Result<World, ImportError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| convert_world(graph))).unwrap_or_else(|panic| {
        let why = panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()));
        Err(ImportError::NotASave(why.unwrap_or_else(|| "inconsistent object graph".into())))
    })
}

fn convert_world(graph: &Graph) -> Result<World, ImportError> {
    let root = |index: usize, class: &str| match graph.roots.get(index) {
        Some(&Value::Node(id)) if graph.class_name(graph.node(id).class) == class => Ok(id),
        other => Err(ImportError::NotASave(format!("root {index} is {other:?}, expected {class}"))),
    };
    let game_node = root(0, "best.f")?;
    let job_market_node = root(1, "best.ay")?;
    let ctx = Context::new(graph);
    let all = |arena: Arena| ctx.nodes(arena).to_vec();
    let mut world = World {
        game: convert_game(&ctx, game_node),
        job_market: convert_coach_job_market(&ctx, job_market_node),
        players: all(Arena::Player).into_iter().map(|n| convert_player(&ctx, n)).collect(),
        clubs: all(Arena::Club).into_iter().map(|n| convert_club(&ctx, n)).collect(),
        coaches: all(Arena::Coach).into_iter().map(|n| convert_coach(&ctx, n)).collect(),
        stadiums: all(Arena::Stadium).into_iter().map(|n| convert_stadium(&ctx, n)).collect(),
        matches: all(Arena::Match).into_iter().map(|n| convert_match(&ctx, n)).collect(),
        competitions: all(Arena::Competition).into_iter().map(|n| convert_competition(&ctx, n)).collect(),
        stages: all(Arena::Stage).into_iter().map(|n| convert_stage(&ctx, n)).collect(),
        countries: all(Arena::Country).into_iter().map(|n| convert_country(&ctx, n)).collect(),
        state_leagues: all(Arena::StateLeague).into_iter().map(|n| convert_state_league(&ctx, n)).collect(),
        // the mod's files beside the save (sav/<save>.economia, ...) have no importer: only the
        // user played the mod, and that career is migrated by hand once
        mods: Default::default(),
    };
    // g.b.Af: the records of the players' competitions find their competition again
    world.link_competition_stats();
    Ok(world)
}
