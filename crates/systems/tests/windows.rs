//! The transfer windows' calendar (`bfmod.Janelas.tick`) against the game: the notices and the
//! late pre-season window of `tools/exporter/export.sh clock 145` (the fictional career with
//! the awards and one tick of the mod a day), three seasons whose first visible day is in
//! March. Only the calendar is compared: the AI's deals of the windows are not ported yet.

use std::path::Path;

use brasfoot_model::clock::Clock;
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_simulation::advance;
use brasfoot_systems::ModSystems;
use brasfoot_systems::config::ModConfig;

const AWARDS_ON: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=1\nPROPOSTAS=0\nJANELA_USUARIO=0\n\
EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n\
PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

#[test]
fn the_windows_open_and_close_like_the_game() {
    let start = Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures/sim-start.s22");
    let mut world = brasfoot_persistence::import::import_s22(std::fs::read(start).unwrap()).unwrap();
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut systems = ModSystems::new(ModConfig::parse(AWARDS_ON));
    world.game.interactive = false;
    // SimHarness: a tick on the loaded career, then after every day but a season's last
    systems.main_screen_tick(&mut world, &mut runtime);
    for _ in 0..145 {
        let over = world.game.season_over;
        advance(&mut world, &mut runtime, &mut systems);
        if over || !world.game.season_over {
            systems.main_screen_tick(&mut world, &mut runtime);
        }
    }
    let file = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/clock-after-145.janelas")).unwrap();
    let mut expected: Vec<&str> = file.lines().filter_map(|l| l.strip_prefix("AVISO\t")).collect();
    let mut actual: Vec<&str> = world.mods.windows.notices.iter().map(String::as_str).collect();
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected);
    let pre = file.lines().find_map(|l| l.strip_prefix("PRE\t")).map(|l| {
        let v: Vec<i32> = l.split('\t').map(|x| x.parse().unwrap()).collect();
        (v[0], v[1], v[2])
    });
    assert_eq!(world.mods.windows.late_preseason, pre);
    let current = file.lines().find_map(|l| l.strip_prefix("ATUAL\t")).and_then(|l| l.split('\t').next()).filter(|s| !s.is_empty());
    assert_eq!(world.mods.windows.current.as_deref(), current);
}
