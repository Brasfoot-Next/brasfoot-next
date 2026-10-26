//! The team of the round and of the year (`a.iu`): a league's best eleven of a round
//! (`f.B.zN()`) or of a season (`zI()`, with the year's best player `zM()`), on the pitch in
//! the places {1, 9, 3, 5, 2, 11, 13, 14, 16, 22, 24} (`iu.sy()`).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{CompetitionId, PlayerId};
use brasfoot_model::world::World;
use eframe::egui::{self, Color32, Stroke, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::new_game::country_name;
use crate::screens::pitch::{TOKEN, paint_pitch, slot_center, token};
use crate::screens::widgets;
use crate::{motion, theme};

/// `iu.sy()`: where the eleven stand, in the record's order.
const PLACES: [usize; 11] = [1, 9, 3, 5, 2, 11, 13, 14, 16, 22, 24];

pub struct TeamOfRoundState {
    /// `iu.w`: false the round, true the year.
    year: bool,
    /// An index of the league countries.
    country: usize,
    /// An index of the country's divisions.
    division: usize,
    /// `MG`: the round or the season, an index of the list.
    period: Option<usize>,
}

impl TeamOfRoundState {
    /// The club's league: its country and division, the last round or season there is.
    pub fn new(world: &World, club: brasfoot_model::ids::ClubId, year: bool) -> Self {
        let mut state = TeamOfRoundState { year, country: 0, division: 0, period: None };
        for (i, &k) in world.game.league_countries.iter().enumerate() {
            if let Some(d) = world.country(k).divisions.iter().position(|&c| world.competition(c).league.is_some_and(|s| world.stage(s).clubs.contains(&club))) {
                state.country = i;
                state.division = d;
            }
        }
        state.period = state.list_len(world).checked_sub(1);
        state
    }

    fn league(&self, world: &World) -> Option<CompetitionId> {
        let k = *world.game.league_countries.get(self.country)?;
        world.country(k).divisions.get(self.division).copied()
    }

    fn list_len(&self, world: &World) -> usize {
        self.league(world).map(|c| if self.year { world.competition(c).selections_zy.len() } else { world.competition(c).selections_aaa.len() }).unwrap_or(0)
    }
}

pub fn window(s: &mut CareerScreen, state: &mut TeamOfRoundState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let world = &s.career.world;
    motion::dialog(ctx, "team-of-round", open, vec2(560.0, 700.0), frame, |ui, open| {
        let title = if state.year { "team-of-the-round-title-year" } else { "team-of-the-round-title-round" };
        if widgets::dialog_title(ui, files, None, &t.text(title, None)) {
            *open = false;
        }
        // ue, vv, MG
        ui.horizontal(|ui| {
            let mut country = state.country;
            let code = |i: usize| world.country(world.game.league_countries[i]).country;
            egui::ComboBox::from_id_salt("tor-country").width(150.0).height(400.0).selected_text(country_name(t, code(state.country))).show_ui(ui, |ui| {
                for i in 0..world.game.league_countries.len() {
                    ui.horizontal(|ui| {
                        widgets::flag(ui, files, code(i), 16.0);
                        ui.selectable_value(&mut country, i, country_name(t, code(i)));
                    });
                }
            });
            if country != state.country {
                // iu.mU(): the divisions and periods again
                state.country = country;
                state.division = 0;
                state.period = state.list_len(world).checked_sub(1);
            }
            let divisions = world.country(world.game.league_countries[state.country]).divisions.len();
            let mut division = state.division;
            egui::ComboBox::from_id_salt("tor-division").width(120.0).selected_text(t.text("team-of-the-round-division", Some(&args([("number", state.division + 1)])))).show_ui(ui, |ui| {
                for d in 0..divisions {
                    ui.selectable_value(&mut division, d, t.text("team-of-the-round-division", Some(&args([("number", d + 1)]))));
                }
            });
            if division != state.division {
                // iu.mX(): the last one is picked
                state.division = division;
                state.period = state.list_len(world).checked_sub(1);
            }
            let count = state.list_len(world);
            let label = |i: usize| {
                if state.year {
                    (world.game.start_year + i as i32).to_string()
                } else {
                    t.text("team-of-the-round-round", Some(&args([("number", i + 1)])))
                }
            };
            let shown = state.period.map(label).unwrap_or_default();
            egui::ComboBox::from_id_salt("tor-period").width(120.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
                for i in 0..count {
                    ui.selectable_value(&mut state.period, Some(i), label(i));
                }
            });
        });
        ui.add_space(12.0);
        let Some(league) = state.league(world) else { return };
        let comp = world.competition(league);
        let list = if state.year { &comp.selections_zy } else { &comp.selections_aaa };
        let Some(eleven) = state.period.and_then(|i| list.get(i)) else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| ui.label(egui::RichText::new(t.text("team-of-the-round-none", None)).color(theme::MUTED)));
            return;
        };
        // zM(): the year's best player
        let best: Option<PlayerId> = if state.year { state.period.and_then(|i| comp.players_zx.get(i).copied()) } else { None };
        let height = (ui.available_height() - 8.0).clamp(300.0, 600.0);
        let size = vec2(height * 2.0 / 3.0, height);
        ui.vertical_centered(|ui| {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            paint_pitch(ui.painter(), rect);
            for (i, &p) in eleven.players.iter().enumerate().take(PLACES.len()) {
                let center = slot_center(rect, PLACES[i]);
                let x = world.player(p);
                token(ui.painter(), center, x.position, x.strength, &x.name, best == Some(p), false);
                if let Some(Some(club)) = eleven.clubs.get(i) {
                    let r = egui::Rect::from_center_size(center + vec2(-TOKEN * 0.9, -TOKEN * 0.8), vec2(16.0, 16.0));
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
                    widgets::crest(&mut child, files, world, *club, 16.0);
                }
                if best == Some(p)
                    && let Some(uri) = files.jar_image(ui.ctx(), "aicons/bolaouro_s2.png")
                {
                    egui::Image::new(uri).paint_at(ui, egui::Rect::from_center_size(center + vec2(TOKEN * 0.9, -TOKEN * 0.8), vec2(18.0, 18.0)));
                }
            }
            if let Some(p) = best {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(t.text("team-of-the-round-best-player", Some(&args([("player", world.player(p).name.clone())])))).font(theme::bold(13.0)).color(Color32::from_rgb(0xf5, 0xc5, 0x42)));
            }
        });
    });
}
