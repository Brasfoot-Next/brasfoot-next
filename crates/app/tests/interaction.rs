//! The app answers at once: a click on a squad row selects it in the next frame, wherever on the
//! row it lands (also on its text), and dialogs close with Esc and fade out. Fictional career.

use brasfoot_i18n::{DEFAULT_LOCALE, Translator};
use brasfoot_next::career::Career;
use brasfoot_next::game_files::GameFiles;
use brasfoot_next::{App, theme};
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness() -> Harness<'static, App> {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../simulation/tests/fixtures/human-start.s22");
    let files = GameFiles::locate();
    let scratch = std::env::temp_dir().join(format!("brasfoot-next-interaction-{}", std::process::id()));
    std::fs::create_dir_all(scratch.join("brasfoot-next/saves")).expect("scratch dir");
    // SAFETY: set before anything reads it, in this test binary only
    unsafe { std::env::set_var("XDG_DATA_HOME", &scratch) };
    // both tests import into the same .bfn: one at a time
    static IMPORT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let mut career = {
        let _guard = IMPORT.lock().unwrap_or_else(|e| e.into_inner());
        Career::import(std::path::Path::new(fixture), &files).expect("the fixture imports")
    };
    career.world.mods.master_sponsor.contract = Some(brasfoot_model::mods::MasterContract {
        brand: 0,
        kind: brasfoot_model::mods::MasterKind::Fixed,
        yearly: 10_000_000,
        bonus_top4: 0,
        bonus_league: 0,
        bonus_continental: 0,
        years: 3,
    });
    career.world.mods.master_sponsor.offers.clear();
    let app = App::with_career(Translator::new(DEFAULT_LOCALE), files, career);
    let mut ready = false;
    let mut harness = Harness::builder().with_size(egui::vec2(1360.0, 768.0)).build_ui_state(
        move |ui, app: &mut App| {
            if !ready {
                ui.ctx().set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
                egui_extras::install_image_loaders(ui.ctx());
                theme::apply(ui.ctx());
                ready = true;
                return;
            }
            app.show(ui);
        },
        app,
    );
    harness.run_steps(4);
    harness
}

fn selected(harness: &mut Harness<'static, App>) -> Option<String> {
    let screen = harness.state_mut().career_screen().expect("in a career");
    screen.selected_player.map(|p| screen.career.world.player(p).name.clone())
}

#[test]
fn a_click_on_a_row_selects_it_in_the_next_frame() {
    let mut harness = harness();
    assert_eq!(selected(&mut harness), None);
    harness.get_by_label("Jacques Rivière").click();
    harness.step();
    assert_eq!(selected(&mut harness).as_deref(), Some("Jacques Rivière"));
    // the frame that shows it was asked for, no mouse move needed
    harness.get_by_label("Ugo Gervais").click();
    harness.step();
    assert_eq!(selected(&mut harness).as_deref(), Some("Ugo Gervais"));
}

#[test]
fn esc_closes_a_dialog() {
    let mut harness = harness();
    harness.state_mut().career_screen().expect("in a career").tables_open = true;
    harness.run_steps(20);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").tables_open);
}

#[test]
fn teams_dialog_opens_and_closes_with_esc() {
    let mut harness = harness();
    harness.state_mut().career_screen().expect("in a career").teams_open = true;
    harness.run_steps(20);
    assert!(harness.state_mut().career_screen().expect("in a career").teams_open);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").teams_open);
}

#[test]
fn lineup_dialog_opens_and_closes_with_esc() {
    let mut harness = harness();
    harness.state_mut().career_screen().expect("in a career").lineup_open = true;
    harness.run_steps(20);
    assert!(harness.state_mut().career_screen().expect("in a career").lineup_open);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").lineup_open);
}

#[test]
fn contract_dialog_opens_and_closes_with_esc() {
    let mut harness = harness();
    let player = {
        let screen = harness.state_mut().career_screen().expect("in a career");
        let club = screen.career.human_club().unwrap();
        screen.career.world.club(club).squad[0]
    };
    {
        let screen = harness.state_mut().career_screen().expect("in a career");
        screen.contract_state = Some(brasfoot_next::screens::contract::ContractState::new(&screen.career.world, player));
        screen.contract_open = true;
    }
    harness.run_steps(20);
    assert!(harness.state_mut().career_screen().expect("in a career").contract_open);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").contract_open);
}

#[test]
fn offer_dialog_opens_and_closes_with_esc() {
    let mut harness = harness();
    let player = {
        let screen = harness.state_mut().career_screen().expect("in a career");
        let club = screen.career.human_club().unwrap();
        let other = screen.career.world.game.clubs.iter().copied().find(|&c| c != club).unwrap();
        screen.career.world.club(other).squad[0]
    };
    {
        let screen = harness.state_mut().career_screen().expect("in a career");
        screen.offer_state = Some(brasfoot_next::screens::offer::OfferState::new(&screen.career.world, player));
        screen.offer_open = true;
    }
    harness.run_steps(20);
    assert!(harness.state_mut().career_screen().expect("in a career").offer_open);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").offer_open);
}

#[test]
fn the_round_stays_open_until_it_ends() {
    let mut harness = harness();
    {
        let screen = harness.state_mut().career_screen().expect("in a career");
        if let Ok(lr) = screen.career.start_live_round() {
            screen.round_state = Some(brasfoot_next::screens::round::RoundState::new(lr));
            screen.round_open = true;
        }
    }
    harness.run_steps(20);
    if harness.state_mut().career_screen().expect("in a career").round_open {
        // the round can't be left halfway: Esc does nothing
        harness.key_press(egui::Key::Escape);
        harness.run_steps(20);
        assert!(harness.state_mut().career_screen().expect("in a career").round_open);
    }
}

#[test]
fn youth_dialog_opens_and_closes_with_esc() {
    let mut harness = harness();
    let club = harness.state_mut().career_screen().expect("in a career").club;
    {
        let screen = harness.state_mut().career_screen().expect("in a career");
        screen.youth_state = Some(brasfoot_next::screens::youth::YouthState::new(club));
        screen.youth_open = true;
    }
    harness.run_steps(20);
    assert!(harness.state_mut().career_screen().expect("in a career").youth_open);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").youth_open);
}

#[test]
fn history_dialog_opens_and_closes_with_esc() {
    let mut harness = harness();
    harness.state_mut().career_screen().expect("in a career").history_open = true;
    harness.run_steps(20);
    assert!(harness.state_mut().career_screen().expect("in a career").history_open);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").history_open);
}

#[test]
fn trophies_dialog_opens_and_closes_with_esc() {
    let mut harness = harness();
    let club = harness.state_mut().career_screen().expect("in a career").club;
    {
        let screen = harness.state_mut().career_screen().expect("in a career");
        screen.trophies_club = Some(club);
        screen.trophies_open = true;
    }
    harness.run_steps(20);
    assert!(harness.state_mut().career_screen().expect("in a career").trophies_open);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(20);
    assert!(!harness.state_mut().career_screen().expect("in a career").trophies_open);
}
