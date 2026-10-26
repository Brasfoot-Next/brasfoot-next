//! The words of the performance analysis (`bfmod.Desempenho`): each finding's title and text from
//! the catalog, with its numbers in the locale.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::mods::StaffRole;
use brasfoot_model::world::World;
use brasfoot_systems::performance::{Finding, TacticAdvice, Unlock};

use crate::format;

fn d1(t: &Translator, v: f64) -> String {
    format::decimal(t, v, 1)
}

fn d0(t: &Translator, v: f64) -> String {
    format::decimal(t, v, 0)
}

/// `Comissao.Cargo.rotulo` in lower case, for "Precisa de {cargo} nível N".
pub fn role_lower(t: &Translator, role: StaffRole) -> String {
    t.text(
        match role {
            StaffRole::Analyst => "findings-role-analyst",
            StaffRole::Assistant => "findings-role-assistant",
            StaffRole::FitnessCoach => "findings-role-fitness-coach",
        },
        None,
    )
}

fn unlock_key(u: Unlock) -> &'static str {
    match u {
        Unlock::AnalystStats => "findings-unlock-analyst-stats",
        Unlock::AnalystDetails => "findings-unlock-analyst-details",
        Unlock::AnalystStrength => "findings-unlock-analyst-strength",
        Unlock::AnalystPlayers => "findings-unlock-analyst-players",
        Unlock::AnalystSeason => "findings-unlock-analyst-season",
        Unlock::AssistantSuggestions => "findings-unlock-assistant-suggestions",
        Unlock::AssistantSwaps => "findings-unlock-assistant-swaps",
        Unlock::AssistantAdjustments => "findings-unlock-assistant-adjustments",
        Unlock::AssistantYoungAndOld => "findings-unlock-assistant-young-old",
        Unlock::AssistantGamePlan => "findings-unlock-assistant-game-plan",
        Unlock::AssistantTactic => "findings-unlock-assistant-tactic",
        Unlock::PhysioReturn => "findings-unlock-physio-return",
        Unlock::PhysioHistory => "findings-unlock-physio-history",
        Unlock::PhysioSuspensions => "findings-unlock-physio-suspensions",
        Unlock::PhysioRest => "findings-unlock-physio-rest",
        Unlock::PhysioSchedule => "findings-unlock-physio-schedule",
    }
}

/// `bloqueado(cargo, nivel, oQue)`
pub fn locked(t: &Translator, role: StaffRole, level: i32, what: &str) -> (String, String) {
    (
        t.text("findings-locked-title", Some(&args([("role", role_lower(t, role)), ("level", level.to_string())]))),
        t.text("findings-locked-text", Some(&args([("what", what.to_owned())]))),
    )
}

/// `Motor.NOMES[tipo]`
pub fn slot_name(t: &Translator, kind: usize) -> String {
    t.text(&format!("findings-slot-{kind}"), None)
}

/// `Motor.ATRIBUTOS[tipo]`
fn slot_attributes(t: &Translator, kind: usize) -> String {
    t.text(&format!("findings-slot-attributes-{kind}"), None)
}

/// `Desempenho.medida()`
fn measure(t: &Translator, world: &World) -> String {
    t.text(if world.game.individual_skills { "findings-measure-pitch" } else { "findings-measure-strength" }, None)
}

fn list(t: &Translator, items: impl IntoIterator<Item = String>) -> String {
    items.into_iter().collect::<Vec<_>>().join(&t.text("findings-list-separator", None))
}

/// `Taticas.texto(int[])`
pub fn tactic_text(t: &Translator, tactic: [i32; 3]) -> String {
    t.text(
        "findings-tactic",
        Some(&args([
            ("style", t.text(&format!("findings-style-{}", tactic[0].clamp(0, 2)), None)),
            ("marking", t.text(&format!("findings-marking-lower-{}", tactic[1].clamp(0, 2)), None)),
            ("side", t.text(&format!("findings-side-lower-{}", tactic[2].clamp(0, 1)), None)),
        ])),
    )
}

/// `conselhoTatico`'s text.
fn tactic_advice(t: &Translator, rival: &str, a: &TacticAdvice) -> String {
    let mut s = t.text("findings-tactic-use", Some(&args([("tactic", tactic_text(t, a.best))])));
    match a.predicted {
        Some(r) => {
            let style = t.text(&format!("findings-style-lower-{}", r[0].clamp(0, 2)), None);
            s += &if a.full_guess {
                t.text(
                    "findings-tactic-guess-full",
                    Some(&args([
                        ("rival", rival.to_owned()),
                        ("style", style.clone()),
                        ("marking", t.text(&format!("findings-marking-lower-{}", r[1].clamp(0, 2)), None)),
                        ("side", t.text(&format!("findings-side-lower-{}", r[2].clamp(0, 1)), None)),
                    ])),
                )
            } else {
                t.text("findings-tactic-guess", Some(&args([("rival", rival.to_owned()), ("style", style.clone())])))
            };
            match a.beats {
                Some(true) => s += &t.text("findings-tactic-beats", Some(&args([("mine", t.text(&format!("findings-style-{}", a.best[0].clamp(0, 2)), None)), ("theirs", style)]))),
                Some(false) => s += &t.text("findings-tactic-loses", None),
                None => {}
            }
        }
        None => s += &t.text("findings-tactic-no-guess", None),
    }
    if a.fast_forwards {
        s += &t.text("findings-tactic-fast-forwards", None);
    } else if a.strong_midfield {
        s += &t.text("findings-tactic-strong-midfield", None);
    }
    if a.tough_defence {
        s += &t.text("findings-tactic-tough-defence", None);
    }
    s += &t.text("findings-tactic-window", None);
    if a.automatic {
        s += &t.text("findings-tactic-automatic", None);
    }
    s
}

/// A finding's title and text.
pub fn text(t: &Translator, world: &World, clock: &brasfoot_model::clock::Clock, f: &Finding) -> (String, String) {
    let tt = |key: &str, a: Option<brasfoot_i18n::Args>| t.text(key, a.as_ref());
    let pair = |title: &str, text: &str, a: brasfoot_i18n::Args| (t.text(title, Some(&a)), t.text(text, Some(&a)));
    let unit = || t.text(if world.game.individual_skills { "findings-unit-pitch" } else { "findings-unit-strength" }, None);
    match f {
        Finding::Locked { role, level, unlock } => locked(t, *role, *level, &tt(unlock_key(*unlock), None)),
        Finding::NoGames => (tt("findings-no-games-title", None), tt("findings-no-games-text", None)),
        Finding::PitchStrength { pitch, screen } => pair("findings-pitch-strength-title", "findings-pitch-strength-text", args([("pitch", d0(t, *pitch)), ("screen", d0(t, *screen))])),
        Finding::Decline { recent, games, form, before, before_games } => pair(
            "findings-decline-title",
            "findings-decline-text",
            args([("recent", d1(t, *recent)), ("games", games.to_string()), ("form", form.clone()), ("before", d1(t, *before)), ("before_games", before_games.to_string())]),
        ),
        Finding::Rising { recent, games, form, before } => {
            pair("findings-rising-title", "findings-rising-text", args([("recent", d1(t, *recent)), ("games", games.to_string()), ("form", form.clone()), ("before", d1(t, *before))]))
        }
        Finding::Form { form, recent, games } => pair("findings-form-title", "findings-form-text", args([("form", form.clone()), ("recent", d1(t, *recent)), ("games", games.to_string())])),
        Finding::DefenceLeaking { conceded, before, shots_against, shots_against_before } => {
            let extra = shots_against_before.map(|b| t.text("findings-defence-leaking-more", Some(&args([("before", d1(t, b))])))).unwrap_or_default();
            pair("findings-defence-leaking-title", "findings-defence-leaking-text", args([("conceded", d1(t, *conceded)), ("before", d1(t, *before)), ("shots", d1(t, *shots_against)), ("extra", extra)]))
        }
        Finding::SolidDefence { conceded, games } => pair("findings-solid-defence-title", "findings-solid-defence-text", args([("conceded", d1(t, *conceded)), ("games", games.to_string())])),
        Finding::AttackFaded { goals, before, shots, aim } => {
            let tail = match aim {
                Some((conversion, conversion_before)) => {
                    t.text("findings-attack-faded-aim", Some(&args([("shots", d0(t, *shots)), ("conversion", d0(t, *conversion)), ("before", d0(t, *conversion_before))])))
                }
                None => t.text("findings-attack-faded-shots", Some(&args([("shots", d1(t, *shots))]))),
            };
            pair("findings-attack-faded-title", "findings-attack-faded-text", args([("goals", d1(t, *goals)), ("before", d1(t, *before)), ("tail", tail)]))
        }
        Finding::SharpAttack { goals, games } => pair("findings-sharp-attack-title", "findings-sharp-attack-text", args([("goals", d1(t, *goals)), ("games", games.to_string())])),
        Finding::SterilePossession { possession } => pair("findings-sterile-possession-title", "findings-sterile-possession-text", args([("possession", d0(t, *possession))])),
        Finding::LateCollapse { late, conceded, games } => {
            pair("findings-late-collapse-title", "findings-late-collapse-text", args([("late", late.to_string()), ("conceded", conceded.to_string()), ("games", games.to_string())]))
        }
        Finding::BadStarts { early, conceded } => pair("findings-bad-starts-title", "findings-bad-starts-text", args([("early", early.to_string()), ("conceded", conceded.to_string())])),
        Finding::CostlyRedCards { reds, games, points_lost } => {
            pair("findings-red-cards-title", "findings-red-cards-text", args([("reds", *reds as i64), ("games", *games as i64), ("lost", *points_lost as i64)]))
        }
        Finding::ManyCards { per_game } => pair("findings-many-cards-title", "findings-many-cards-text", args([("cards", d1(t, *per_game))])),
        Finding::AbsencesInDefeats { defeats, average } => pair("findings-absences-title", "findings-absences-text", args([("defeats", defeats.to_string()), ("average", d1(t, *average))])),
        Finding::InjuredStarters { players } => pair("findings-injured-starters-title", "findings-injured-starters-text", args([("players", list(t, players.iter().cloned())), ("count", players.len().to_string())])),
        Finding::Underperforming { expected, games, points } => pair(
            "findings-underperforming-title",
            "findings-underperforming-text",
            args([("measure", measure(t, world)), ("expected", d0(t, *expected)), ("games", games.to_string()), ("points", points.to_string())]),
        ),
        Finding::Overperforming { points, games, expected } => pair(
            "findings-overperforming-title",
            "findings-overperforming-text",
            args([("measure", measure(t, world)), ("expected", d0(t, *expected)), ("games", games.to_string()), ("points", points.to_string())]),
        ),
        Finding::AsExpected { points, games, expected } => pair(
            "findings-as-expected-title",
            "findings-as-expected-text",
            args([("measure", measure(t, world)), ("expected", d0(t, *expected)), ("games", games.to_string()), ("points", points.to_string())]),
        ),
        Finding::ToughRun { recent, season } => pair("findings-tough-run-title", "findings-tough-run-text", args([("measure", measure(t, world)), ("recent", d0(t, *recent)), ("season", d0(t, *season))])),
        Finding::WeakerLineups { recent, season } => pair("findings-weaker-lineups-title", "findings-weaker-lineups-text", args([("measure", measure(t, world)), ("recent", d0(t, *recent)), ("season", d0(t, *season))])),
        Finding::WeakAway { home, away } => pair("findings-weak-away-title", "findings-home-away-text", args([("home", d1(t, *home)), ("away", d1(t, *away))])),
        Finding::WeakHome { home, away } => pair("findings-weak-home-title", "findings-home-away-text", args([("home", d1(t, *home)), ("away", d1(t, *away))])),
        Finding::LowCohesion { changes, players, games } => {
            pair("findings-low-cohesion-title", "findings-low-cohesion-text", args([("changes", d1(t, *changes)), ("players", players.to_string()), ("games", games.to_string())]))
        }
        Finding::ScorerDependence { player, goals, total } => {
            pair("findings-scorer-dependence-title", "findings-scorer-dependence-text", args([("player", player.clone()), ("goals", goals.to_string()), ("total", total.to_string())]))
        }
        Finding::PlayersInSlump { average, players } => {
            let names = list(t, players.iter().map(|(n, r)| t.text("findings-slump-player", Some(&args([("player", n.clone()), ("rating", d1(t, *r))])))));
            pair("findings-slump-title", "findings-slump-text", args([("average", d1(t, *average)), ("players", names)]))
        }
        Finding::StrikerWithoutGoals { player } => pair("findings-striker-title", "findings-striker-text", args([("player", player.clone())])),
        Finding::Reinforce { position, starters, team } => pair(
            "findings-reinforce-title",
            "findings-reinforce-text",
            args([("position", t.text(&format!("findings-position-{}", position.clamp(&0, &4)), None)), ("starters", d0(t, *starters)), ("team", d0(t, *team))]),
        ),
        Finding::ReinforceSlot { kind, slot, team } => pair(
            "findings-reinforce-title",
            "findings-reinforce-slot-text",
            args([("position", slot_name(t, *kind)), ("slot", d0(t, *slot)), ("team", d0(t, *team)), ("attributes", slot_attributes(t, *kind))]),
        ),
        Finding::PlayInSlot { player, kind, value, starter, current, screen } => {
            let tail = screen.map(|(a, b)| t.text("findings-play-in-slot-screen", Some(&args([("mine", a.to_string()), ("theirs", b.to_string()), ("attributes", slot_attributes(t, *kind))])))).unwrap_or_default();
            pair(
                "findings-play-in-slot-title",
                "findings-play-in-slot-text",
                args([("player", player.clone()), ("slot", slot_name(t, *kind)), ("value", value.to_string()), ("unit", unit()), ("starter", starter.clone()), ("current", current.to_string()), ("tail", tail)]),
            )
        }
        Finding::OutOfPlace { player, value, kind, best_kind, best } => pair(
            "findings-out-of-place-title",
            "findings-out-of-place-text",
            args([("player", player.clone()), ("value", value.to_string()), ("unit", unit()), ("slot", slot_name(t, *kind)), ("best_slot", slot_name(t, *best_kind)), ("best", best.to_string())]),
        ),
        Finding::RightLineup => (tt("findings-right-lineup-title", None), tt("findings-right-lineup-text", None)),
        Finding::ChangeEarlier => (tt("findings-change-earlier-title", None), tt("findings-change-earlier-text", None)),
        Finding::ProtectDefence => (tt("findings-protect-defence-title", None), tt("findings-protect-defence-text", None)),
        Finding::FocusAttacks => (tt("findings-focus-attacks-title", None), tt("findings-focus-attacks-text", None)),
        Finding::MoreDirect => (tt("findings-more-direct-title", None), tt("findings-more-direct-text", None)),
        Finding::LessAggressive => (tt("findings-less-aggressive-title", None), tt("findings-less-aggressive-text", None)),
        Finding::SettleTeam => (tt("findings-settle-team-title", None), tt("findings-settle-team-text", None)),
        Finding::GiveMinutes { player, age, value } => {
            pair("findings-give-minutes-title", "findings-give-minutes-text", args([("player", player.clone()), ("age", age.to_string()), ("measure", measure(t, world)), ("value", value.to_string())]))
        }
        Finding::PlanExit { player, age, strength } => {
            pair("findings-plan-exit-title", "findings-plan-exit-text", args([("player", player.clone()), ("age", age.to_string()), ("measure", measure(t, world)), ("strength", strength.to_string())]))
        }
        Finding::GamePlan { rival, plan } => {
            let name = world.club(*rival).name.clone();
            (t.text("findings-game-plan-title", Some(&args([("rival", name.clone())]))), t.text(&format!("findings-game-plan-{plan}"), Some(&args([("rival", name)]))))
        }
        Finding::Tactic { rival, advice } => {
            let name = world.club(*rival).name.clone();
            (t.text("findings-tactic-title", Some(&args([("rival", name.clone())]))), tactic_advice(t, &name, advice))
        }
        Finding::NoInjured => (tt("findings-no-injured-title", None), tt("findings-no-injured-text", None)),
        Finding::Injured { players } => {
            let names = list(
                t,
                players.iter().map(|(n, back)| match back {
                    Some(d) => {
                        let f = clock.fields(*d);
                        t.text("findings-injured-back", Some(&args([("player", n.clone()), ("date", format!("{:02}/{:02}", f.day, f.month + 1))])))
                    }
                    None => n.clone(),
                }),
            );
            (t.text("findings-injured-title", Some(&args([("count", players.len() as i64)]))), t.text("findings-injured-text", Some(&args([("players", names)]))))
        }
        Finding::InjuryHistory { total, prone } => {
            let title = t.text("findings-injury-history-title", Some(&args([("total", *total as i64)])));
            let text = if prone.is_empty() {
                t.text("findings-injury-history-none", None)
            } else {
                let names = list(t, prone.iter().map(|(n, c)| t.text("findings-injury-history-player", Some(&args([("player", n.clone()), ("count", c.to_string())])))));
                t.text("findings-injury-history-prone", Some(&args([("players", names)])))
            };
            (title, text)
        }
        Finding::NextMatch { stage, competition, suspended, booked } => {
            let name = match (stage, competition) {
                (Some(s), _) => brasfoot_i18n::labels::collapse_spaces(&crate::screens::widgets::stage_name(t, world, *s)),
                (None, Some(c)) => crate::screens::widgets::competition_name(t, world, *c),
                _ => String::new(),
            };
            let mut text = if suspended.is_empty() { t.text("findings-next-match-none", None) } else { t.text("findings-next-match-suspended", Some(&args([("players", list(t, suspended.iter().cloned()))]))) };
            if !booked.is_empty() {
                text += &t.text("findings-next-match-booked", Some(&args([("players", list(t, booked.iter().cloned()))])));
            }
            (t.text("findings-next-match-title", Some(&args([("competition", name)]))), text)
        }
        Finding::RestBooked { first, more } => {
            (tt("findings-rest-booked-title", None), t.text(if *more { "findings-rest-booked-more" } else { "findings-rest-booked-one" }, Some(&args([("player", first.clone())]))))
        }
        Finding::TightSchedule { games } => pair("findings-tight-schedule-title", "findings-tight-schedule-text", args([("games", games.to_string())])),
    }
}
