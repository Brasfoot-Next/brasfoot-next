//! Creates a career from a game folder's data files (read only) with some leagues chosen and
//! plays a few days, to find what is not ported yet.
//! Usage: new_game_smoke <game folder> <days> <country ids...>

use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;
use brasfoot_simulation::new_game::NewGame;

/// Every mod system off but the tactics (`SimHarness.SYSTEMS_OFF`).
const SYSTEMS_OFF: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=0\nPROPOSTAS=0\nJANELA_USUARIO=0\n\
EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n\
PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("game folder"));
    let days: u32 = args.next().expect("days").parse().unwrap();
    let chosen: Vec<i32> = args.map(|a| a.parse().unwrap()).collect();
    let templates = brasfoot_persistence::game_data::read_teams(&dir.join("teams"));
    let configs = brasfoot_persistence::game_data::read_league_configs(&dir.join("conf_ligas_nacionais"));
    let mut options = brasfoot_persistence::game_data::read_options(&dir.join("options.bcf"));
    options.plays_state = false;
    options.plays_regionals = false;
    options.plays_international_clubs = false;
    options.plays_all_national_teams = false;
    let mut new_game = NewGame::new(templates, configs);
    for l in &mut new_game.leagues {
        l.selected = chosen.contains(&l.country);
    }
    let mut runtime = Runtime::new(RandomSource::from_clock());
    runtime.names = brasfoot_model::names::NameLists::new(format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap()));
    let mut world = World::default();
    let mut systems = brasfoot_systems::ModSystems::new(brasfoot_systems::config::ModConfig::parse(SYSTEMS_OFF));
    let started = std::time::Instant::now();
    {
        let mut sim = Sim { world: &mut world, runtime: &mut runtime, hooks: &mut systems, human: None };
        sim.start_new_world(&options);
        sim.create_world(&new_game, &options, 2022);
        sim.find_next_day();
    }
    println!("created in {:?}: {} clubs, {} players, {} days", started.elapsed(), world.clubs.len(), world.players.len(), world.game.days.len());
    world.game.interactive = false;
    for d in 0..days {
        brasfoot_simulation::advance(&mut world, &mut runtime, &mut systems);
        if d % 10 == 0 {
            println!("day {} season {}", world.game.day, world.season());
        }
    }
    println!("ok after {days} days");
}
