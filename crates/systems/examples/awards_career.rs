//! Plays the fictional career of the tests (`crates/simulation/tests/fixtures/sim-start.s22`)
//! with the awards' effects on and writes it as a save of the app, to look at the awards window
//! (`BF_OPEN=<file> BF_TAB=awards`). Usage: awards_career <out.bfn> [days, default 242].

use brasfoot_model::clock::Clock;
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_persistence::native::{FORMAT_VERSION, SaveMeta, write_save};
use brasfoot_systems::ModSystems;
use brasfoot_systems::config::ModConfig;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).expect("the .bfn to write");
    let days: u32 = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(242);
    let start = concat!(env!("CARGO_MANIFEST_DIR"), "/../simulation/tests/fixtures/sim-start.s22");
    let mut world = brasfoot_persistence::import::import_s22(std::fs::read(start).unwrap()).unwrap();
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.names = NameLists::new(format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap()));
    let config = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=1\nPROPOSTAS=0\nJANELA_USUARIO=0\nEMPRESTIMOS_IA=0\n\
PEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\nPATRIMONIO=0\nESTADIOS=0\n\
CREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";
    let mut systems = ModSystems::new(ModConfig::parse(config));
    world.game.interactive = false;
    for _ in 0..days {
        brasfoot_simulation::advance(&mut world, &mut runtime, &mut systems);
    }
    world.number_players_for_save();
    world.number_competitions_for_save();
    let meta = SaveMeta {
        format_version: FORMAT_VERSION,
        app_version: String::new(),
        club: String::new(),
        coach: String::new(),
        date: String::new(),
        season: world.season(),
        saved_at: 0,
        imported_from: None,
    };
    write_save(std::path::Path::new(out), &meta, &world).unwrap();
    println!("{out}: season {}, {} awards", world.season(), world.mods.awards.awards.len());
}
