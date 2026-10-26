//! Loads a save (a copy: the file is only read) and plays some days with the mod's systems, to
//! find what is not ported yet for a real career.
//! Usage: save_smoke <save.s22> <days> [bfmod.config]   (without a config every system is off
//! but the tactics, as in the differential tests)

use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;

const SYSTEMS_OFF: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=0\nPROPOSTAS=0\nJANELA_USUARIO=0\n\
EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n\
PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("save file");
    let days: u32 = args.next().expect("days").parse().unwrap();
    let config = match args.next() {
        Some(c) => std::fs::read_to_string(c).unwrap(),
        None => SYSTEMS_OFF.to_owned(),
    };
    let started = std::time::Instant::now();
    let mut world = brasfoot_persistence::import::import_s22(std::fs::read(&path).unwrap()).unwrap();
    println!("loaded in {:?}: season {} day {}, {} clubs, {} players", started.elapsed(), world.season(), world.game.day, world.clubs.len(), world.players.len());
    if std::env::var("BF_LIST_LEAGUES").is_ok() {
        // the national leagues with playoffs: relegation not all direct, or a promotion knockout
        for &country in &world.game.league_countries {
            for &l in &world.country(country).divisions {
                let c = world.competition(l);
                let t = world.stage(c.league.unwrap());
                if t.knockout_teams > 0 || t.format_code == 1020 || t.format_code == 1068 || t.group_count > 1 {
                    println!("country {} division {} table format {} groups {} knockout teams {} clubs {}", world.country(country).country, c.division, t.format_code, t.group_count, t.knockout_teams, t.clubs.len());
                }
                if c.relegated != c.relegated_directly || c.promotion_knockout_places > 0 {
                    println!("country {} division {} clubs {} relegated {} direct {} promotion knockout {}", world.country(country).country, c.division, world.stage(c.league.unwrap()).clubs.len(), c.relegated, c.relegated_directly, c.promotion_knockout_places);
                }
            }
        }
    }
    let mut runtime = Runtime::new(RandomSource::from_clock());
    runtime.names = brasfoot_model::names::NameLists::new(format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap()));
    let mut systems = brasfoot_systems::ModSystems::new(brasfoot_systems::config::ModConfig::parse(&config));
    world.game.interactive = false;
    let started = std::time::Instant::now();
    // the mod's clock, as the main screen runs it after every day but a season's last
    systems.main_screen_tick(&mut world, &mut runtime);
    for d in 0..days {
        let over = world.game.season_over;
        brasfoot_simulation::advance(&mut world, &mut runtime, &mut systems);
        if over || !world.game.season_over {
            systems.main_screen_tick(&mut world, &mut runtime);
        }
        // the offers for the human squad, refused
        systems.squad_offers.clear();
        if d % 10 == 0 {
            println!("day {} season {} ({:?})", world.game.day, world.season(), started.elapsed());
        }
    }
    println!("ok after {days} days in {:?}", started.elapsed());
}
