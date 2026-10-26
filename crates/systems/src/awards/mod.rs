// Port of bfmod.BolaDeOuro: the season-end awards (Ballon d'Or, Golden Shoe, Golden Glove,
// Golden Boy, team of the year, coach of the year) and the Hall of Fame. The patched season-end
// screen a.cR hands the season over when it opens, right after f.al().

pub mod clubs;
pub mod coaches;
pub mod hall_of_fame;
pub mod history;
pub mod season;

use brasfoot_model::world::World;

use crate::Mod;
use crate::config::{ModConfig, Switch};
use season::Season;

/// `BolaDeOuro.fimDeTemporada(Window, boolean)`: records the awards, applies their effects and
/// brings the Hall of Fame and the club history up to date, once per season and run
/// (`ultimaTemporadaAnunciada` is static, so a save loaded again at the end of a season gets
/// its effects a second time, as in the original).
pub fn season_end(m: &mut Mod) {
    let s = m.world.season();
    if s == m.sys.last_awards_season {
        return;
    }
    m.sys.last_awards_season = s;
    let (world, config) = (&mut *m.world, &m.sys.config);
    let t = Season::current(world);
    if t.ballon_dor(1).is_empty() {
        return;
    }
    let coaches = coaches::ranking(world, &t);
    history::register(world, &t, &coaches, true);
    apply_effects(world, config, &t);
    // Jornal.lendas: the news of the legends who retired (the newspaper is not ported)
    let _retired = hall_of_fame::update(world, t.season);
    clubs::register(world, &t, true);
    m.super_mundial_season_end();
    clubs::update_idols(m.world);
    m.stadiums_season_end();
    // Jornal.premiacao: the awards' news is not ported
    m.master_sponsor_season_end(&t);
    m.investor_season_end(&t);
    m.sponsor_slots_season_end(&t);
    m.youth_league_season_end();
    m.assets_season_end();
}

/// `BolaDeOuro.abrir`: the awards window opened from its buttons. Careers and idols move only
/// when a day is played, so they are brought up to date once per save and calendar day
/// (`historicoEm`); a season just started, with nobody eligible yet, shows the one before.
pub fn open_history(world: &mut World, refreshed_at: &mut Option<(Option<String>, i32)>) -> Season {
    let today = (world.game.save_name.clone(), world.game.day);
    if refreshed_at.as_ref() != Some(&today) {
        // Jornal.lendas: the news of the legends who retired (the newspaper is not ported)
        let _retired = hall_of_fame::update(world, world.season() - 1);
        clubs::update_idols(world);
        *refreshed_at = Some(today);
    }
    let t = Season::current(world);
    if t.ballon_dor(1).is_empty() && world.season() > 1 {
        return Season::new(world, world.season() - 1);
    }
    t
}

/// `aplicarEfeitos(Temporada)`: the Ballon d'Or winner becomes world class, the podium and the
/// other winners stars (the game's own flags: more market value, a small bonus in matches); the
/// coach of the year gains a reputation level.
fn apply_effects(world: &mut World, config: &ModConfig, t: &Season) {
    if !config.on(Switch::AwardEffects) {
        return;
    }
    let podium: Vec<_> = t.ballon_dor(3).iter().map(|c| c.player).collect();
    for (i, &p) in podium.iter().enumerate() {
        if i == 0 {
            world.player_mut(p).world_class = true;
        }
        world.player_mut(p).set_star(true);
    }
    let winners: Vec<_> = [t.golden_shoe(1), t.golden_glove(world, 1), t.golden_boy(1)].iter().filter_map(|w| w.first().map(|c| c.player)).collect();
    for p in winners {
        world.player_mut(p).set_star(true);
    }
    if let Some(best) = coaches::ranking(world, t).first() {
        let coach = world.coach_mut(best.coach);
        coach.reputation = (coach.reputation + 1).min(5);
    }
}
