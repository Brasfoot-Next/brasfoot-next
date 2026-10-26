//! Differential test of a career with a human club, against the game. The fixtures come from
//! `tools/exporter/export.sh human-days <days>` (exporter.HumanHarness): the fictional career of
//! the simulation tests with a made-up human coach at club 1, played with the game's screens.
//! The human of the harness (exporter.HumanPolicy) renews the contracts about to end, picks the
//! 4-4-2 of the line-up screen and confirms, and changes nothing during the matches; the
//! functions below do the same.

mod common;

use std::path::Path;

use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::messages::{blank_composed_texts, fill_date_texts};
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_simulation::human::contract::{self, Answer};
use brasfoot_simulation::human::lineup::LineupScreen;
use brasfoot_model::ids::CoachId;
use brasfoot_simulation::human::HumanDecisions;
use brasfoot_engine::hooks::MatchHooks;
use brasfoot_simulation::{Sim, SimulationHooks, advance_with};
use brasfoot_systems::ModSystems;
use brasfoot_systems::config::{ModConfig, Switch};
use brasfoot_i18n::Translator;
use brasfoot_model::mods::{LoanRole, Pillar, SponsorSlot, StaffRole};

/// The `bfmod.config` of the harness (`SimHarness.SYSTEMS_OFF`).
const SYSTEMS_OFF: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=0\nPROPOSTAS=0\nJANELA_USUARIO=0\n\
EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n\
PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

/// `HumanPolicy.FORMATION`: "4-4-2" in the combo of the line-up screen.
const FORMATION: usize = 5;
/// `HumanPolicy.RENEWAL_HORIZON`
const RENEWAL_HORIZON: i64 = 30 * 86_400_000;
/// "1 ano"
const ONE_YEAR: usize = 1;

fn load(name: &str) -> World {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures").join(name);
    brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap()
}

fn first_difference(actual: &str, expected: &str) -> Option<String> {
    let a: Vec<&str> = actual.lines().collect();
    let e: Vec<&str> = expected.lines().collect();
    let i = a.iter().zip(&e).position(|(x, y)| x != y).or_else(|| (a.len() != e.len()).then(|| a.len().min(e.len())))?;
    let path: Vec<&str> = e[..i].iter().rev().filter(|l| l.starts_with('.')).take(6).copied().collect();
    let show = |lines: &[&str]| lines[i.saturating_sub(4)..(i + 4).min(lines.len())].join("\n");
    Some(format!("line {}, fields {:?}\n--- Rust\n{}\n--- Java\n{}", i + 1, path, show(&a), show(&e)))
}

/// `HumanPolicy.renewContracts`: one year at the current salary (below the limit) for every
/// contract ending within 30 days, at the salary the player asks if he refuses.
fn renew_contracts(world: &mut World, club: ClubId) {
    let now = world.today_millis();
    for p in world.club(club).squad.clone() {
        if world.player(p).contract_until >= now + RENEWAL_HORIZON {
            continue;
        }
        let limit = contract::salary_limit(world, p);
        let salary = world.player(p).salary.min(limit - 1);
        if salary <= 0 {
            continue;
        }
        if let Answer::Asks(asked) = contract::offer(world, p, salary, ONE_YEAR) {
            contract::sign(world, p, asked, ONE_YEAR);
        }
    }
}

/// `HumanPolicy.lineUp`: the 4-4-2, or else the first formation of the combo with eleven.
fn line_up(world: &mut World, club: ClubId, m: MatchId) {
    let mut screen = LineupScreen::open(world, club, m);
    let order = std::iter::once(FORMATION).chain((2..13).filter(|&f| f != FORMATION));
    for formation in order {
        screen.pick_formation(world, formation);
        if screen.starters() == 11 {
            screen.confirm(world).unwrap();
            return;
        }
    }
    panic!("no formation fills eleven places for {}", world.club(club).name);
}

/// `HumanPolicy` in the dialogs: no change during the matches; without a club the coach takes
/// the first club that wants him (`jobOffers`), with one he turns them down.
struct HarnessHuman;

impl HumanDecisions for HarnessHuman {
    fn job_offer(&mut self, world: &World, coach: CoachId, clubs: &[ClubId]) -> Option<ClubId> {
        if world.coach(coach).club_id.is_none() { clubs.first().copied() } else { None }
    }
}

/// `HumanPolicy.STAFF_LEVEL`
const STAFF_LEVEL: i32 = 3;

/// `HumanPolicy.hireStaff`: for each role without someone of `STAFF_LEVEL` or more, the first
/// candidate of the month at that level who accepts the club.
fn hire_staff(world: &mut World, runtime: &mut Runtime, systems: &mut ModSystems) {
    for role in StaffRole::ALL {
        if world.mods.staff.hired(role).is_some_and(|m| m.level >= STAFF_LEVEL) {
            continue;
        }
        let candidates = systems.staff_candidates(world, runtime);
        for (i, p) in candidates.iter().enumerate() {
            if p.role == role && p.level >= STAFF_LEVEL && systems.hire_staff(world, runtime, i).is_ok() {
                break;
            }
        }
    }
}

/// `HumanPolicy.sponsorSlots`: in the slots' order, the first offer of a slot that has offers is
/// signed; an empty slot nobody is looking for gets the commercial department.
fn sponsor_slots(world: &mut World, runtime: &mut Runtime, systems: &mut ModSystems) {
    for slot in SponsorSlot::ALL {
        let state = &world.mods.sponsor_slots;
        if let Some(i) = state.offers.iter().position(|o| o.slot == slot) {
            systems.with(world, runtime, |m| m.sign_sponsor_slot(i));
        } else if !state.deals.iter().any(|d| d.slot == slot) && !state.searches.iter().any(|s| s.0 == slot) {
            systems.with(world, runtime, |m| m.search_sponsor_slot(slot)).unwrap();
        }
    }
}

/// The mod's dialogs the tick opened (`HumanPolicy.later`, run after it): the mixed master
/// offer is signed, the investor accepted.
fn answer_dialogs(world: &mut World, runtime: &mut Runtime, systems: &mut ModSystems) {
    if !world.mods.master_sponsor.offers.is_empty() {
        let offer = 1.min(world.mods.master_sponsor.offers.len() - 1);
        systems.with(world, runtime, |m| m.sign_master_sponsor(offer));
    }
    if world.mods.investor.shown.is_some() {
        systems.with(world, runtime, |m| m.accept_investor());
    }
    // HumanPolicy.acceptSquadOffer: every offer and loan request, each accepted one bringing the
    // main screen (and the mod's clock) back
    for offer in std::mem::take(&mut systems.squad_offers) {
        if systems.with(world, runtime, |m| m.answer_squad_offer(offer, true)) {
            systems.main_screen_tick(world, runtime);
            answer_dialogs(world, runtime, systems);
        }
    }
}

/// `HumanPolicy.investInAssets`: the first pillar whose next level costs at most half the cash;
/// with no work going on and over 60 mi, a stadium expansion; the first naming-rights offer when
/// there is none; the national campaign when allowed and at most a third of the cash.
fn invest_in_assets(world: &mut World, runtime: &mut Runtime, systems: &mut ModSystems, club: ClubId) {
    use brasfoot_systems::assets::LEVEL_COST;
    for pillar in Pillar::ALL {
        let n = systems.with(world, runtime, |m| m.pillar_level(club, pillar));
        if n < 5 && LEVEL_COST[n as usize + 1] * 2 <= world.club(club).cash {
            let _ = systems.with(world, runtime, |m| m.upgrade_pillar(club, pillar));
            break;
        }
    }
    let stadium = world.club(club).stadium;
    if stadium.is_some() && !world.game.stadium_works.iter().any(|w| w.stadium == stadium) && world.club(club).cash > 60_000_000 {
        let _ = systems.with(world, runtime, |m| m.start_stadium_expansion(club, [500, 2000, 300, 20]));
    }
    let e = systems.with(world, runtime, |m| m.assets_data(club));
    if e.naming_brand.is_empty() {
        let offers = systems.with(world, runtime, |m| m.naming_offers(club));
        if let Some(o) = offers.first() {
            let _ = systems.with(world, runtime, |m| m.sign_naming(club, o));
        }
    }
    if e.marketing >= 1 && 20_000_000 * 3 <= world.club(club).cash {
        let _ = systems.with(world, runtime, |m| m.run_campaign(club, 1));
    }
}

/// `HumanPolicy.bank`: with no loan, 80% of the available credit over 12 months.
fn bank(world: &mut World, runtime: &mut Runtime, systems: &mut ModSystems, club: ClubId) {
    systems.with(world, runtime, |m| {
        if m.bank_loans(club).is_empty() {
            let amount = m.available_credit(club) * 8 / 10;
            if amount > 0 {
                m.borrow(club, amount, 12);
            }
        }
    });
}

/// `HumanPolicy.satellites`: with no satellite, a partnership with the first AI club that may be
/// one, outside his division; with a satellite and no loanee there, the youngest player not on loan goes as a regular.
fn satellites(world: &mut World, runtime: &mut Runtime, systems: &mut ModSystems, club: ClubId) {
    systems.with(world, runtime, |m| {
        let mine = m.satellites_of(club);
        let Some(&satellite) = mine.first() else {
            for t in brasfoot_systems::economy::ai_clubs(m.world) {
                if m.block_reason(club, t, false).is_none() && !brasfoot_systems::satellites::same_division(m.world, club, t) {
                    let _ = m.sign_partnership(club, t);
                    return;
                }
            }
            return;
        };
        if m.world.club(satellite).squad.iter().any(|&p| brasfoot_systems::loans::loan_of(m.world, p).is_some()) {
            return;
        }
        let mut youngest: Option<PlayerId> = None;
        for &p in &m.world.club(club).squad {
            if !m.world.player(p).on_loan && youngest.is_none_or(|y| m.world.player(p).age < m.world.player(y).age) {
                youngest = Some(p);
            }
        }
        if let Some(p) = youngest {
            let _ = m.lend_to_satellite(p, satellite, LoanRole::Regular);
        }
    });
}

/// `HumanPolicy.promoteJuniors`: every junior of 18 or more, in the list's order, while the
/// squad has room.
fn promote_juniors(world: &mut World, runtime: &mut Runtime, systems: &mut ModSystems, club: ClubId) {
    for p in world.game.youth_players.clone() {
        if world.player(p).club != Some(club) || world.player(p).age < 18 {
            continue;
        }
        if systems.promote_junior(world, runtime, club, p).is_err() {
            return;
        }
    }
}

/// `HumanHarness.playDay`
/// The mod without its season end: without `BF_CLOCK` the harness's season-end screen is only a
/// dialog (`human-patch/best/S.aY`), so `BolaDeOuro.fimDeTemporada` never runs; the rest of the
/// mod steps in as in the game.
struct NoSeasonEnd(ModSystems);

impl MatchHooks for NoSeasonEnd {
    fn tactics_enabled(&self) -> bool {
        self.0.tactics_enabled()
    }
    fn on_kickoff(&mut self, world: &mut World, runtime: &mut Runtime, m: MatchId) {
        self.0.on_kickoff(world, runtime, m)
    }
    fn tactic_directive(&self, world: &World, club: ClubId) -> Option<[i32; 3]> {
        self.0.tactic_directive(world, club)
    }
    fn on_tick(&mut self, world: &mut World, runtime: &mut Runtime, m: MatchId) {
        self.0.on_tick(world, runtime, m)
    }
    fn match_bonus(&self, world: &World, m: MatchId, side: usize) -> f64 {
        self.0.match_bonus(world, m, side)
    }
    fn auto_tactics(&self, world: &World) -> bool {
        self.0.auto_tactics(world)
    }
    fn analyst_level(&self, world: &World) -> i32 {
        self.0.analyst_level(world)
    }
    fn prepared_tactic(&self, world: &World, rival: ClubId) -> Option<([i32; 3], i32)> {
        self.0.prepared_tactic(world, rival)
    }
}

impl SimulationHooks for NoSeasonEnd {
    fn game_buyer(&mut self, world: &mut World, runtime: &mut Runtime, player: PlayerId, drawn: Option<ClubId>) -> Option<ClubId> {
        self.0.game_buyer(world, runtime, player, drawn)
    }
    fn approve_game_sale(&mut self, world: &mut World, runtime: &mut Runtime, player: PlayerId, buyer: ClubId, fee: i32) -> bool {
        self.0.approve_game_sale(world, runtime, player, buyer, fee)
    }
}

/// The hooks of a run: the whole mod with `BF_CLOCK`, the mod without its season end without.
enum Hooks {
    /// The mod, and what the human does besides playing.
    Clock(ModSystems, Policy),
    NoClock(NoSeasonEnd),
}

impl Hooks {
    fn hooks(&mut self) -> &mut dyn SimulationHooks {
        match self {
            Hooks::Clock(h, _) => h,
            Hooks::NoClock(h) => h,
        }
    }
}

fn play_day(world: &mut World, runtime: &mut Runtime, hooks: &mut Hooks) {
    if world.game.season_over {
        advance_with(world, runtime, hooks.hooks(), &mut HarnessHuman);
        return;
    }
    let today = world.game.days[world.game.day as usize].matches.clone();
    let human: Vec<MatchId> = today
        .into_iter()
        .filter(|&m| {
            let g = world.game_match(m);
            [g.home, g.away].into_iter().flatten().any(|c| world.club(c).human)
        })
        .collect();
    if human.is_empty() {
        advance_with(world, runtime, hooks.hooks(), &mut HarnessHuman);
        return;
    }
    if let Hooks::Clock(systems, policy) = hooks {
        if systems.config.on(Switch::Staff) {
            hire_staff(world, runtime, systems);
        }
        if policy.sponsors {
            sponsor_slots(world, runtime, systems);
        }
        if policy.assets {
            for &m in &human {
                let g = world.game_match(m);
                for club in [g.home, g.away].into_iter().flatten() {
                    if world.club(club).human {
                        invest_in_assets(world, runtime, systems, club);
                    }
                }
            }
        }
        for (on, act) in [(policy.bank, bank as fn(&mut World, &mut Runtime, &mut ModSystems, ClubId)), (policy.satellites, satellites)] {
            if !on {
                continue;
            }
            for &m in &human {
                let g = world.game_match(m);
                for club in [g.home, g.away].into_iter().flatten() {
                    if world.club(club).human {
                        act(world, runtime, systems, club);
                    }
                }
            }
        }
        if policy.juniors {
            for &m in &human {
                let g = world.game_match(m);
                for club in [g.home, g.away].into_iter().flatten() {
                    if world.club(club).human {
                        promote_juniors(world, runtime, systems, club);
                    }
                }
            }
        }
    }
    for &m in &human {
        let g = world.game_match(m);
        for club in [g.home, g.away].into_iter().flatten() {
            if world.club(club).human && !world.club(club).lineup_set {
                renew_contracts(world, club);
                line_up(world, club, m);
            }
        }
    }
    Sim { world, runtime, hooks: hooks.hooks(), human: Some(&mut HarnessHuman) }.play_human_day();
}

fn compare(days: u32) {
    run(days, None, false);
}

fn run(days: u32, market: Option<&str>, staff: bool) {
    run_with(days, market, Policy { staff, ..Default::default() }, &[]);
}

/// What the harness's human does besides playing (`BF_STAFF`, `BF_JUNIORS`, `BF_SPONSORS`), and
/// the day the mod's test command brings an investor (`BF_INVESTOR_DAY`).
#[derive(Clone, Copy, Default)]
struct Policy {
    staff: bool,
    juniors: bool,
    sponsors: bool,
    assets: bool,
    bank: bool,
    satellites: bool,
    investor_day: Option<u32>,
}

/// `HumanHarness` with `BF_CLOCK` (and `BF_MARKET`): the awards at each season end and the mod's
/// clock after every other day; the fixtures are in this crate's.
/// `HumanHarness` with `BF_SYSTEMS` (the switches turned on) and `BF_STAFF`.
fn run_with(days: u32, market: Option<&str>, policy: Policy, switches_on: &[&str]) {
    let Policy { staff, juniors, sponsors, assets, bank, satellites, investor_day } = policy;
    let mut world = load("human-start.s22");
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let config = match market {
        None => SYSTEMS_OFF.to_owned(),
        Some(m) => SYSTEMS_OFF.replace("EFEITOS_PREMIOS=0", "EFEITOS_PREMIOS=1").replace("MERCADO=DESLIGADO", &format!("MERCADO={m}")),
    };
    // HumanHarness with BF_STAFF
    let mut config = if staff { config.replace("COMISSAO=0", "COMISSAO=1").replace("TATICA_AUTO=0", "TATICA_AUTO=1") } else { config };
    for k in switches_on {
        config = config.replace(&format!("\n{k}=0"), &format!("\n{k}=1"));
    }
    let systems = ModSystems::new(ModConfig::parse(&config));
    let mut hooks = if market.is_some() { Hooks::Clock(systems, policy) } else { Hooks::NoClock(NoSeasonEnd(systems)) };
    world.game.interactive = false;
    runtime.trace_seeds = std::env::var("BF_TRACE_RNG").is_ok();
    if let Hooks::Clock(systems, _) = &mut hooks {
        systems.main_screen_tick(&mut world, &mut runtime);
        answer_dialogs(&mut world, &mut runtime, systems);
    }
    for day in 0..days {
        let over = world.game.season_over;
        play_day(&mut world, &mut runtime, &mut hooks);
        if let Hooks::Clock(systems, policy) = &mut hooks
            && policy.investor_day == Some(day + 1)
        {
            systems.with(&mut world, &mut runtime, |m| m.request_investor_offer());
        }
        if let Hooks::Clock(systems, _) = &mut hooks
            && (over || !world.game.season_over)
        {
            systems.main_screen_tick(&mut world, &mut runtime);
            answer_dialogs(&mut world, &mut runtime, systems);
        }
        // HumanHarness with BF_TRACE_DAYS
        if std::env::var("BF_TRACE_DAYS").is_ok()
            && let Some(&coach) = world.game.coaches.iter().find(|&&c| match std::env::var("BF_TRACE_COACH") {
                Ok(name) => world.coach(c).name.ends_with(&name),
                Err(_) => world.coach(c).human,
            })
        {
            let c = world.coach(coach);
            let cash = c.club_id.map(|t| format!(", cash {}", world.club(t).cash)).unwrap_or_default();
            println!("after {} days: board {}, fans {}{cash}", day + 1, c.board_confidence, c.fan_support);
        }
    }
    // the harness saves the result and the exporter loads it (g.b.Ai, g.b.Aj)
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    blank_composed_texts(&mut world);
    fill_date_texts(&mut world, &runtime.clock);
    let switches: String = switches_on.iter().map(|k| format!("{}-", k.to_lowercase())).collect();
    let prefix = format!(
        "human-clock-{switches}{}{}{}{}{}{}{}market-after-{days}",
        if staff { "staff-" } else { "" },
        if juniors { "juniors-" } else { "" },
        if sponsors { "sponsors-" } else { "" },
        if assets { "assets-" } else { "" },
        if bank { "bank-" } else { "" },
        if satellites { "satellites-" } else { "" },
        investor_day.map(|d| format!("investor{d}-")).unwrap_or_default()
    );
    let mut java = match market {
        None => load(&format!("human-after-{days}.s22")),
        Some(_) => brasfoot_persistence::import::import_s22(std::fs::read(fixture(&format!("{prefix}.s22"))).unwrap()).unwrap(),
    };
    blank_composed_texts(&mut java);
    let expected = canonical(&java);
    if let Ok(dir) = std::env::var("BF_DUMP") {
        std::fs::write(Path::new(&dir).join(format!("rust-{days}.txt")), canonical(&world)).unwrap();
        std::fs::write(Path::new(&dir).join(format!("java-{days}.txt")), &expected).unwrap();
        let meta = brasfoot_persistence::native::SaveMeta::default();
        brasfoot_persistence::native::write_save(&Path::new(&dir).join(format!("rust-{days}.bfn")), &meta, &world).unwrap();
    }
    if let Some(difference) = first_difference(&canonical(&world), &expected) {
        panic!("after {days} days: {difference}");
    }
    if market.is_some() {
        let t = Translator::new(brasfoot_i18n::DEFAULT_LOCALE);
        let java = std::fs::read_to_string(fixture(&format!("{prefix}.economia"))).unwrap();
        let expected: String = java.lines().filter(|l| !l.starts_with("TRANSF\t") && !l.starts_with("JANELA\t")).map(|l| format!("{l}\n")).collect();
        // the .economia of Economia.tick's end: the later systems of the tick (the bank's
        // instalments, the loans' fines) change the fortunes in memory only
        let saved = match &hooks {
            Hooks::Clock(systems, _) => systems.economy_saved.clone(),
            Hooks::NoClock(_) => None,
        };
        let ours = match &saved {
            Some(e) => common::economy_file_of(&t, &world, e).0,
            None => common::economy_file(&t, &world).0,
        };
        if let Some(d) = first_difference(&ours, &expected) {
            panic!("after {days} days, .economia: {d}");
        }
        let expected = std::fs::read_to_string(fixture(&format!("{prefix}.comissao"))).unwrap_or_default();
        if let Some(d) = first_difference(&common::staff_file(&t, &world), &expected) {
            panic!("after {days} days, .comissao: {d}");
        }
        let expected = std::fs::read_to_string(fixture(&format!("{prefix}.base"))).unwrap_or_default();
        if let Some(d) = first_difference(&common::academy_file(&world), &expected) {
            panic!("after {days} days, .base: {d}");
        }
        let expected = std::fs::read_to_string(fixture(&format!("{prefix}.joias"))).unwrap_or_default();
        if let Some(d) = first_difference(&common::gems_file(&world), &expected) {
            panic!("after {days} days, .joias: {d}");
        }
        for (ext, ours) in [
            (".patrocinio", common::master_sponsor_file(&t, &world)),
            (".cotas", common::sponsor_slots_file(&t, &world)),
            (".investidor", common::investor_file(&world)),
            (".patrimonio", common::assets_file(&world)),
        ] {
            let mut expected = std::fs::read_to_string(fixture(&format!("{prefix}{ext}"))).unwrap_or_default();
            // Cotas.fimDeTemporada saves even with the system off: no club is no file here
            if expected == "CLUBE\t-1\n" {
                expected.clear();
            }
            if let Some(d) = first_difference(&ours, &expected) {
                panic!("after {days} days, {ext}: {d}");
            }
        }
        if let Ok(java) = std::fs::read_to_string(fixture(&format!("{prefix}.satelites")))
            && let Some(d) = first_difference(&common::satellites_file(&t, &world), &common::satellites_java(&java))
        {
            panic!("after {days} days, .satelites: {d}");
        }
        if let Ok(java) = std::fs::read_to_string(fixture(&format!("{prefix}.credito")))
            && let Some(d) = first_difference(&common::credit_file(&world), &common::credit_java(&java))
        {
            panic!("after {days} days, .credito: {d}");
        }
        if let Ok(java) = std::fs::read_to_string(fixture(&format!("{prefix}.base_camp")))
            && let Some(d) = first_difference(&common::youth_league_file(&t, &world), &common::youth_league_java(&java))
        {
            panic!("after {days} days, .base_camp: {d}");
        }
        if let Ok(java) = std::fs::read_to_string(fixture(&format!("{prefix}.base_hist")))
            && let Some(d) = first_difference(&common::youth_league_history_file(&t, &world), &java)
        {
            panic!("after {days} days, .base_hist: {d}");
        }
        if let Ok(java) = std::fs::read_to_string(fixture(&format!("{prefix}.emprestimos")))
            && let Some(d) = first_difference(&common::loans_file(&world), &java)
        {
            panic!("after {days} days, .emprestimos: {d}");
        }
        let expected = std::fs::read_to_string(fixture(&format!("{prefix}.torcida"))).unwrap_or_default();
        if let Some(d) = first_difference(&common::fans_file(&t, &world, &runtime.clock), &expected) {
            panic!("after {days} days, .torcida: {d}");
        }
    }
}

/// The human club's first match.
#[test]
fn first_day_matches_the_game() {
    compare(1);
}

/// Ten match days of the human club.
#[test]
fn ten_days_match_the_game() {
    compare(10);
}

/// The day the human coach is sacked.
#[test]
fn sacking_matches_the_game() {
    compare(32);
}

/// The next day: he takes the first club that wants him.
#[test]
fn new_job_matches_the_game() {
    compare(33);
}

/// The whole first season.
#[test]
fn first_season_matches_the_game() {
    compare(48);
}

/// The new season's first day, with the human club's year.
#[test]
fn second_season_starts_like_the_game() {
    compare(49);
}

/// The second season.
#[test]
fn second_season_matches_the_game() {
    compare(97);
}

/// The third season: sacked again, and a new club.
#[test]
fn third_season_matches_the_game() {
    compare(145);
}

/// Three seasons with the awards and the mod's clock, the market on: the economy with a human
/// club that changes three times, and the windows.
#[test]
fn three_seasons_with_the_mods_clock_match_the_game() {
    run(145, Some("REALISTA"), false);
}

/// The same three seasons with the coaching staff and the automatic tactics: the human hires an
/// analyst, an assistant and a fitness coach of level 3 or more (`HumanPolicy.hireStaff`).
#[test]
fn three_seasons_with_a_coaching_staff_match_the_game() {
    run(145, Some("REALISTA"), true);
}

/// The youth academy with a human club: his kids are only reported, the gems revealed each
/// season (`Joias`).
#[test]
fn three_seasons_with_the_youth_academy_and_a_human_club_match_the_game() {
    run_with(145, Some("REALISTA"), Policy { staff: true, ..Default::default() }, &["BASE"]);
}

/// The fans' mood (`Torcida`) on top of the academy, the human promoting his juniors of 18 or
/// more (`HumanPolicy.promoteJuniors`): kids breaking through, and the members' monthly income.
#[test]
fn three_seasons_with_the_fans_match_the_game() {
    run_with(145, Some("REALISTA"), Policy { staff: true, juniors: true, ..Default::default() }, &["BASE", "TORCIDA"]);
}

/// The human club's sponsorship and owner (`Patrocinio`, `Cotas`, `Investidor`): the human signs
/// the mixed master offer, works every slot (`HumanPolicy.sponsorSlots`) and sells the club to
/// the investor the mod's test command brings on day 20 (`BF_INVESTOR_DAY`).
#[test]
fn three_seasons_with_sponsors_and_investors_match_the_game() {
    let policy = Policy { staff: true, juniors: true, sponsors: true, investor_day: Some(20), ..Default::default() };
    run_with(145, Some("REALISTA"), policy, &["BASE", "TORCIDA", "PATROCINIO", "INVESTIDOR"]);
}

/// The club's infrastructure with a human club that builds (`HumanPolicy.investInAssets`): the
/// upkeep, the stadium works, naming rights, the campaign, the youth trials and the human's
/// academy, on top of every system so far.
#[test]
fn three_seasons_with_the_clubs_infrastructure_match_the_game() {
    let policy = Policy { staff: true, juniors: true, sponsors: true, assets: true, investor_day: Some(20), ..Default::default() };
    run_with(145, Some("REALISTA"), policy, &["BASE", "TORCIDA", "PATROCINIO", "INVESTIDOR", "PATRIMONIO"]);
}

/// The bank with a human club (`Credito`): the human borrows 80% of his limit over a year
/// whenever he owes nothing (`HumanPolicy.bank`), and pays the instalments every month.
#[test]
fn three_seasons_with_the_bank_and_a_human_club_match_the_game() {
    run_with(145, Some("REALISTA"), Policy { staff: true, bank: true, ..Default::default() }, &["CREDITO"]);
}

/// The national U-20 league (`CampeonatoBase`) on top of the academy: the youth XIs played by
/// the engine on the senior league's days, the standouts' development, the podium's money.
#[test]
fn three_seasons_with_the_youth_league_match_the_game() {
    run_with(145, Some("REALISTA"), Policy { staff: true, ..Default::default() }, &["BASE", "CAMPEONATO_BASE"]);
}

/// The rich clubs' offers for the human's starters and the loan requests for his benched
/// youngsters in the windows (`PROPOSTAS`, `PEDIDOS_EMPRESTIMO`), all accepted.
#[test]
fn three_seasons_with_offers_for_the_human_squad_match_the_game() {
    run_with(145, Some("REALISTA"), Policy { staff: true, ..Default::default() }, &["PROPOSTAS", "PEDIDOS_EMPRESTIMO"]);
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Satellites with a human club (`Satelites`), with the bank and the AI's loans: the human signs
/// a partnership with the first club of another division that may be his satellite and lends it his youngest player
/// (`HumanPolicy.satellites`).
#[test]
fn three_seasons_with_a_human_satellite_network_match_the_game() {
    let policy = Policy { staff: true, bank: true, satellites: true, ..Default::default() };
    run_with(145, Some("REALISTA"), policy, &["SATELITES", "CREDITO", "EMPRESTIMOS_IA"]);
}
