//! Opens `.bfn` saves (read only) and reports their format version and whether they load.
//! Usage: open_bfn <save.bfn>...

fn main() {
    for path in std::env::args().skip(1) {
        let path = std::path::PathBuf::from(path);
        let meta = brasfoot_persistence::native::read_meta(&path).map(|m| m.format_version);
        match brasfoot_persistence::native::read_save::<brasfoot_model::world::World>(&path) {
            Ok((_, world)) => {
                println!("{}: format {:?}, ok, season {}, {} clubs", path.display(), meta.ok(), world.season(), world.clubs.len());
                let g = &world.game;
                for (name, c) in [("europa_league", g.europa_league), ("conference_league", g.conference_league), ("copa_sudamericana", g.copa_sudamericana), ("finalissima", g.finalissima)] {
                    if let Some(c) = c {
                        let c = world.competition(c);
                        println!("  {name}: {:?} kind {}", c.class, c.kind);
                    }
                }
            }
            Err(e) => println!("{}: format {:?}, error {e}", path.display(), meta.ok()),
        }
    }
}
