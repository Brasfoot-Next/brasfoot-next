//! Debug aid for the second-season differential: plays the fixture's days with the mod and
//! lists where the transfer history, the youth list and the players first differ from the game.
//! Usage: compare_season <days>

use std::path::Path;

use brasfoot_model::clock::Clock;
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_systems::ModSystems;
use brasfoot_systems::config::ModConfig;

const SYSTEMS_OFF: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=0\nPROPOSTAS=0\nJANELA_USUARIO=0\nEMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\nPATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

fn load(name: &str) -> brasfoot_model::world::World {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures").join(name);
    brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap()
}

fn main() {
    let days: u32 = std::env::args().nth(1).and_then(|d| d.parse().ok()).unwrap_or(49);
    let mut world = load("sim-start.s22");
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.names = NameLists::new(format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap()));
    let mut systems = ModSystems::new(ModConfig::parse(SYSTEMS_OFF));
    world.game.interactive = false;
    for _ in 0..days {
        brasfoot_simulation::advance(&mut world, &mut runtime, &mut systems);
    }
    let java = load(&format!("sim-after-{days}.s22"));
    let name = |w: &brasfoot_model::world::World, p: Option<brasfoot_model::ids::PlayerId>| p.map(|p| w.player(p).name.clone()).unwrap_or_default();
    println!("transfers rust {} java {}", world.game.transfers.len(), java.game.transfers.len());
    for (i, (a, b)) in world.game.transfers.iter().zip(&java.game.transfers).enumerate() {
        let (x, y) = ((name(&world, a.player), a.from_club, a.to_club, a.fee), (name(&java, b.player), b.from_club, b.to_club, b.fee));
        if x != y {
            println!("first different transfer {i}: rust {x:?} java {y:?}");
            for k in i.saturating_sub(3)..(i + 6).min(world.game.transfers.len()).min(java.game.transfers.len()) {
                let (a, b) = (&world.game.transfers[k], &java.game.transfers[k]);
                println!("  {k}: rust {} {}->{} {} | java {} {}->{} {}", name(&world, a.player), a.from_club, a.to_club, a.fee, name(&java, b.player), b.from_club, b.to_club, b.fee);
            }
            break;
        }
    }
    println!("players rust {} java {}; youth rust {} java {}", world.game.players.len(), java.game.players.len(), world.game.youth_players.len(), java.game.youth_players.len());
    for (i, (&a, &b)) in world.game.players.iter().zip(&java.game.players).enumerate() {
        let (x, y) = (world.player(a), java.player(b));
        if x.name != y.name || x.age != y.age || x.strength != y.strength {
            println!("first different player {i}: rust {} {} {} java {} {} {}", x.name, x.age, x.strength, y.name, y.age, y.strength);
            break;
        }
    }
    for (i, (&a, &b)) in world.game.youth_players.iter().zip(&java.game.youth_players).enumerate() {
        let (x, y) = (world.player(a), java.player(b));
        if x.name != y.name || x.age != y.age || x.potential != y.potential {
            println!("first different youth {i}: rust {} {} {} java {} {} {}", x.name, x.age, x.potential, y.name, y.age, y.potential);
            break;
        }
    }
    println!("coaches rust {} java {}", world.game.coaches.len(), java.game.coaches.len());
    let mut shown = 0;
    for (&a, &b) in world.game.players.iter().zip(&java.game.players) {
        let (x, y) = (world.player(a), java.player(b));
        if x.salary != y.salary && shown < 8 {
            shown += 1;
            let club = |w: &brasfoot_model::world::World, p: &brasfoot_model::saved::Player| p.club.map(|c| (w.club(c).id, w.club(c).division, w.club(c).level)).unwrap_or_default();
            println!("salary {} rust {} java {} | strength {}/{} age {}/{} pos {} star {}/{} wc {}/{} youth {}/{} club {:?}/{:?}", x.name, x.salary, y.salary, x.strength, y.strength, x.age, y.age, x.position, x.star, y.star, x.world_class, y.world_class, x.youth, y.youth, club(&world, x), club(&java, y));
        }
    }
}
