//! Text of the structured names the model keeps (`brasfoot_model::labels`).

use brasfoot_model::countries;
use brasfoot_model::labels::{CompetitionLabel, StageDivision, StageLabel};
use brasfoot_model::mods::{AwardKind, CoachSummary, RecordKind, SeasonTitle, TeamSlot};

use crate::{Translator, args};

/// The country's name from the catalog (`country-<code>`).
pub fn country(t: &Translator, id: i32) -> String {
    let code = countries::country(id).map(|k| k.code).unwrap_or("").to_lowercase();
    t.text(&format!("country-{code}"), None)
}

/// `at.getNome()`
pub fn competition(t: &Translator, label: &CompetitionLabel) -> String {
    match label {
        CompetitionLabel::Saved(name) => name.clone(),
        CompetitionLabel::Stage(stage) => self::stage(t, stage),
        CompetitionLabel::SuperCup { country } => t.text("competition-super-cup", Some(&args([("country", self::country(t, *country))]))),
        CompetitionLabel::League { country, division } => {
            t.text("competition-league", Some(&args([("country", self::country(t, *country)), ("division", division.to_string())])))
        }
        // f.f: "Copa " + ad.bq(country) + " " + its name
        CompetitionLabel::Cup { country } => t.text("competition-cup", Some(&args([("article", cup_article(t, *country)), ("country", self::country(t, *country))]))),
        CompetitionLabel::ClubWorldCup => t.text("competition-club-world-cup", None),
        CompetitionLabel::Intercontinental => t.text("competition-intercontinental", None),
        CompetitionLabel::FriendlyTournament => t.text("competition-friendly-tournament", None),
        CompetitionLabel::SuperMundial => t.text("competition-super-mundial", None),
        CompetitionLabel::Unnamed => t.text("competition-unnamed", None),
    }
}

/// `ad.br(int)`: "Campeonato " and the country's adjective (`ad.bp`), or the article of its
/// cup (`ad.bq`) and its name.
pub fn national_league(t: &Translator, country: i32) -> String {
    let adjective_key = format!("country-adjective-{country}");
    let adjective = t.text(&adjective_key, None);
    if adjective != adjective_key {
        return t.text("league-name-adjective", Some(&args([("adjective", adjective)])));
    }
    t.text("league-name-country", Some(&args([("article", cup_article(t, country)), ("country", self::country(t, country))])))
}

/// `ad.bq(int)`: the article before the country's name ("do Brasil"), "da" by default.
pub fn cup_article(t: &Translator, country: i32) -> String {
    let key = format!("country-cup-article-{country}");
    let article = t.text(&key, None);
    if article != key { article } else { t.text("country-cup-article-default", None) }
}

/// `f.s.getNome()`: cut to 39 characters (UTF-16 units, as `String.length()`) and "...".
pub fn stage(t: &Translator, label: &StageLabel) -> String {
    let division = match &label.division {
        StageDivision::None => String::new(),
        StageDivision::Number(d) => t.text("stage-division-suffix", Some(&args([("division", d.to_string())]))),
        StageDivision::Named(name) => t.text("stage-division-named", Some(&args([("name", name.clone())]))),
    };
    let name = match label.league_of {
        // the composed name gets the cuts the saved ones got (Campeonato dos EUA: "s EUA", as in the original)
        Some(country) if label.name.is_empty() => {
            brasfoot_model::labels::CUT_WORDS.iter().fold(national_league(t, country), |name, cut| name.replace(cut, ""))
        }
        _ => label.name.clone(),
    };
    let full = format!("{name}{division}");
    let units: Vec<u16> = full.encode_utf16().collect();
    if units.len() > 39 { format!("{}...", String::from_utf16_lossy(&units[..39])) } else { full }
}

/// `replaceAll("\\s+", " ").trim()`: what the mod does to the labels it builds from names.
pub fn collapse_spaces(text: &str) -> String {
    text.split([' ', '\t', '\n', '\u{0B}', '\u{0C}', '\r']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// `Temporada.adicionar`: "Campeão <competition>" or "Vice <competition>", spaces collapsed.
pub fn season_title(t: &Translator, title: &SeasonTitle) -> String {
    let key = if title.runner_up { "award-title-runner-up" } else { "award-title-champion" };
    collapse_spaces(&t.text(key, Some(&args([("competition", competition(t, &title.competition))]))))
}

/// The titles joined by " · " (`String.join(" · ", titulos)`).
pub fn season_titles(t: &Translator, titles: &[SeasonTitle]) -> String {
    titles.iter().map(|x| season_title(t, x)).collect::<Vec<_>>().join(" · ")
}

/// A league's name as the mod keeps it, spaces collapsed.
pub fn league(t: &Translator, label: &CompetitionLabel) -> String {
    collapse_spaces(&competition(t, label))
}

/// `Tecnicos.Candidato.resumo()`: "1º no <league> (previsto 3º)" and the titles.
pub fn coach_summary(t: &Translator, summary: &CoachSummary) -> String {
    let mut parts = Vec::new();
    if let Some(k) = &summary.campaign {
        let a = args([("position", k.position.to_string()), ("league", league(t, &k.league)), ("expected", k.expected.to_string())]);
        parts.push(t.text("award-coach-campaign", Some(&a)));
    }
    parts.extend(summary.titles.iter().map(|x| season_title(t, x)));
    parts.join(" · ")
}

/// The place's name in the team of the year (`Temporada.selecao`).
pub fn team_slot(t: &Translator, slot: TeamSlot) -> String {
    let key = match slot {
        TeamSlot::Goalkeeper => "award-slot-goalkeeper",
        TeamSlot::RightBack => "award-slot-right-back",
        TeamSlot::CentreBack => "award-slot-centre-back",
        TeamSlot::LeftBack => "award-slot-left-back",
        TeamSlot::Midfielder => "award-slot-midfielder",
        TeamSlot::Forward => "award-slot-forward",
    };
    t.text(key, None)
}

/// `Clubes.artilheiros`: "<name> (<goals>)".
pub fn club_top_scorer(t: &Translator, name: &str, goals: i32) -> String {
    t.text("award-club-top-scorer", Some(&args([("name", name.to_string()), ("goals", goals.to_string())])))
}

fn award_kind_key(kind: AwardKind) -> &'static str {
    match kind {
        AwardKind::BallonDor => "ballon-dor",
        AwardKind::GoldenShoe => "golden-shoe",
        AwardKind::GoldenGlove => "golden-glove",
        AwardKind::GoldenBoy => "golden-boy",
        AwardKind::TeamOfTheYear => "team-of-the-year",
        AwardKind::CoachOfTheYear => "coach-of-the-year",
    }
}

/// `Premios.Tipo.nome`
pub fn award_kind(t: &Translator, kind: AwardKind) -> String {
    t.text(&format!("award-kind-{}", award_kind_key(kind)), None)
}

/// `Premios.resumo`: "2 Bolas · 1 Chuteira", in the kinds' order.
pub fn award_counts(t: &Translator, years: &std::collections::BTreeMap<AwardKind, Vec<i32>>) -> String {
    years
        .iter()
        .map(|(kind, list)| t.text(&format!("award-count-{}", award_kind_key(*kind)), Some(&args([("count", list.len())]))))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// `HallDaFama.Recorde.nome`
pub fn record_kind(t: &Translator, kind: RecordKind) -> String {
    let key = match kind {
        RecordKind::Goals => "record-goals",
        RecordKind::Assists => "record-assists",
        RecordKind::Contributions => "record-contributions",
        RecordKind::Rating => "record-rating",
    };
    t.text(key, None)
}



/// `Economia.Clube.tipoPatrocinio`; empty before the first sponsor.
pub fn sponsorship_kind(t: &Translator, kind: brasfoot_model::mods::SponsorshipKind) -> String {
    use brasfoot_model::mods::SponsorshipKind::*;
    let key = match kind {
        None => return String::new(),
        Fixed => "economy-sponsorship-fixed",
        Mixed => "economy-sponsorship-mixed",
        Risky => "economy-sponsorship-risky",
        OwnerLinked => "economy-sponsorship-owner-linked",
    };
    t.text(key, Option::None)
}


/// `Comissao.PERFIS`
pub fn staff_profile(t: &Translator, profile: u16) -> String {
    t.text(&format!("staff-profile-{profile}"), None)
}

/// The key of `Comissao.Cargo.rotulo`.
pub fn staff_role_key(role: brasfoot_model::mods::StaffRole) -> &'static str {
    use brasfoot_model::mods::StaffRole::*;
    match role {
        Analyst => "staff-role-analyst",
        Assistant => "staff-role-assistant",
        FitnessCoach => "staff-role-fitness-coach",
    }
}

/// `Torcida.motivos[i][2]`: why the fans' mood moved.
pub fn fan_event(t: &Translator, why: &brasfoot_model::mods::FanEvent) -> String {
    use brasfoot_model::mods::FanEvent::*;
    let player = |key: &str, p: &str| t.text(key, Some(&args([("player", p.to_owned())])));
    match why {
        AcademyPromotion { player: p } => player("fans-reason-academy-promotion", p),
        WorldStarSigned { player: p } => player("fans-reason-world-star-signed", p),
        ImpactSigning { player: p } => player("fans-reason-impact-signing", p),
        IdolSold { player: p, club } => t.text("fans-reason-idol-sold", Some(&args([("player", p.clone()), ("club", club.clone())]))),
        TopPlayerLeft { player: p } => player("fans-reason-top-player-left", p),
        NewOwner { investor } => t.text("fans-reason-new-owner", Some(&args([("investor", brasfoot_model::brands::owner(*investor))]))),
        OwnerGaveUp => t.text("fans-reason-owner-gave-up", None),
        ArenaComfort => t.text("fans-reason-arena-comfort", None),
        Inauguration { pillar, level } => t.text("fans-reason-inauguration", Some(&args([("facility", pillar_level(t, *pillar, *level))]))),
        NamingRightsSold { stadium } => t.text("fans-reason-naming-rights-sold", Some(&args([("stadium", stadium.clone())]))),
        Campaign { kind } => t.text(&format!("fans-reason-campaign-{kind}"), None),
        StadiumRenamed { stadium } => t.text("fans-reason-stadium-renamed", Some(&args([("stadium", stadium.clone())]))),
        YouthLeagueTitle => t.text("fans-reason-youth-league-title", None),
        YouthLeagueRunnerUp => t.text("fans-reason-youth-league-runner-up", None),
    }
}



/// `Cotas.Cota.rotulo`, and its lower case.
pub fn sponsor_slot(t: &Translator, slot: brasfoot_model::mods::SponsorSlot, lower: bool) -> String {
    use brasfoot_model::mods::SponsorSlot::*;
    let key = match slot {
        Sleeve => "sponsor-slot-sleeve",
        Back => "sponsor-slot-back",
        Shorts => "sponsor-slot-shorts",
        Kit => "sponsor-slot-kit",
        Stadium => "sponsor-slot-stadium",
        Partner => "sponsor-slot-partner",
    };
    t.text(&if lower { format!("{key}-lower") } else { key.to_owned() }, None)
}

fn sponsor_years(t: &Translator, years: i32) -> String {
    t.text("sponsor-years", Some(&args([("years", years as i64)])))
}

/// `Patrocinio.descricao(Contrato)`; `millions` writes an amount as `Patrocinio.mi` does.
pub fn master_description(t: &Translator, k: &brasfoot_model::mods::MasterContract, millions: &dyn Fn(i64) -> String) -> String {
    let years = sponsor_years(t, k.years);
    if k.bonus_top4 > 0 {
        let a = args([
            ("yearly", millions(k.yearly)),
            ("years", years),
            ("top4", millions(k.bonus_top4)),
            ("league", millions(k.bonus_league)),
            ("continental", millions(k.bonus_continental)),
        ]);
        t.text("sponsor-master-description-bonus", Some(&a))
    } else {
        t.text("sponsor-master-description", Some(&args([("yearly", millions(k.yearly)), ("years", years)])))
    }
}

/// `Patrocinio.Contrato.tipo`
pub fn master_kind(t: &Translator, kind: brasfoot_model::mods::MasterKind) -> String {
    use brasfoot_model::mods::MasterKind::*;
    t.text(
        match kind {
            Fixed => "sponsor-master-kind-fixed",
            Mixed => "sponsor-master-kind-mixed",
            Risky => "sponsor-master-kind-risky",
        },
        None,
    )
}

/// `Patrocinio.historico`
pub fn master_note(t: &Translator, note: &brasfoot_model::mods::MasterNote, millions: &dyn Fn(i64) -> String) -> String {
    use brasfoot_model::mods::{MasterBonusReason, MasterNote::*};
    match note {
        Signed { year, contract } => {
            let a = args([
                ("year", year.to_string()),
                ("brand", brasfoot_model::brands::MASTER_BRANDS[contract.brand as usize].to_owned()),
                ("kind", master_kind(t, contract.kind)),
                ("description", master_description(t, contract, millions)),
            ]);
            t.text("sponsor-master-note-signed", Some(&a))
        }
        Bonus { year, amount, reasons } => {
            let reasons: Vec<String> = reasons
                .iter()
                .map(|r| match r {
                    MasterBonusReason::Place(p) => t.text("sponsor-master-bonus-place", Some(&args([("place", p.to_string())]))),
                    MasterBonusReason::LeagueTitle => t.text("sponsor-master-bonus-league", None),
                    MasterBonusReason::ContinentalTitle => t.text("sponsor-master-bonus-continental", None),
                })
                .collect();
            let a = args([("year", year.to_string()), ("amount", millions(*amount)), ("reasons", reasons.join(&t.text("sponsor-list-separator", None)))]);
            t.text("sponsor-master-note-bonus", Some(&a))
        }
        Ended { year, brand, fine } => {
            let a = args([("year", year.to_string()), ("brand", brasfoot_model::brands::MASTER_BRANDS[*brand as usize].to_owned()), ("fine", millions(*fine))]);
            t.text("sponsor-master-note-ended", Some(&a))
        }
    }
}

/// `Cotas.Contrato.tipo`
pub fn slot_kind(t: &Translator, kind: brasfoot_model::mods::SlotKind) -> String {
    use brasfoot_model::mods::SlotKind::*;
    t.text(
        match kind {
            Fixed => "sponsor-slot-kind-fixed",
            Short => "sponsor-slot-kind-short",
            WithBonus => "sponsor-slot-kind-with-bonus",
        },
        None,
    )
}

/// `Cotas.descricao(Contrato)`
pub fn slot_description(t: &Translator, k: &brasfoot_model::mods::SlotContract, millions: &dyn Fn(i64) -> String) -> String {
    let years = sponsor_years(t, k.years);
    if k.bonus > 0 {
        t.text("sponsor-slot-description-bonus", Some(&args([("yearly", millions(k.yearly)), ("years", years), ("bonus", millions(k.bonus))])))
    } else {
        t.text("sponsor-slot-description", Some(&args([("yearly", millions(k.yearly)), ("years", years)])))
    }
}

/// `Cotas.historico`
pub fn slot_note(t: &Translator, note: &brasfoot_model::mods::SlotNote, millions: &dyn Fn(i64) -> String) -> String {
    use brasfoot_model::mods::SlotNote::*;
    match note {
        Signed { year, contract } => {
            let a = args([
                ("year", year.to_string()),
                ("slot", sponsor_slot(t, contract.slot, false)),
                ("brand", brasfoot_model::brands::slot_brand(contract.brand)),
                ("kind", slot_kind(t, contract.kind)),
                ("description", slot_description(t, contract, millions)),
            ]);
            t.text("sponsor-slot-note-signed", Some(&a))
        }
        TitleBonus { year, amount } => t.text("sponsor-slot-note-title-bonus", Some(&args([("year", year.to_string()), ("amount", millions(*amount))]))),
        Ended { year, slot, brand, fine } => {
            let a = args([("year", year.to_string()), ("slot", sponsor_slot(t, *slot, true)), ("brand", brasfoot_model::brands::slot_brand(*brand)), ("fine", millions(*fine))]);
            t.text("sponsor-slot-note-ended", Some(&a))
        }
    }
}

/// The key part of a `Patrimonio.Pilar`.
fn pillar_key(pillar: brasfoot_model::mods::Pillar) -> &'static str {
    use brasfoot_model::mods::Pillar::*;
    match pillar {
        Arena => "arena",
        TrainingCentre => "training-centre",
        Academy => "academy",
        Marketing => "marketing",
    }
}

/// `Pilar.titulo`
pub fn pillar(t: &Translator, pillar: brasfoot_model::mods::Pillar) -> String {
    t.text(&format!("assets-pillar-{}", pillar_key(pillar)), None)
}

/// `Pilar.nomes[level]`
pub fn pillar_level(t: &Translator, pillar: brasfoot_model::mods::Pillar, level: i32) -> String {
    t.text(&format!("assets-pillar-{}-level-{level}", pillar_key(pillar)), None)
}

/// `Pilar.descricoes[level]`
pub fn pillar_effect(t: &Translator, pillar: brasfoot_model::mods::Pillar, level: i32) -> String {
    t.text(&format!("assets-pillar-{}-effect-{level}", pillar_key(pillar)), None)
}

/// The key of a line of the satellites' history (`Satelites.registrarHistorico`'s type).
pub fn satellite_event_key(event: &brasfoot_model::mods::SatelliteEvent) -> &'static str {
    use brasfoot_model::mods::SatelliteEvent::*;
    match event {
        Partnership { .. } => "satellite-event-partnership",
        Renewal { .. } => "satellite-event-renewal",
        Termination { .. } => "satellite-event-termination",
        Acquisition { .. } => "satellite-event-acquisition",
        ClubSale { .. } => "satellite-event-club-sale",
        CashInjection { .. } => "satellite-event-cash-injection",
        ProfitWithdrawal { .. } => "satellite-event-profit-withdrawal",
        Assets { .. } => "satellite-event-assets",
        Loan { .. } => "satellite-event-loan",
        Return { .. } => "satellite-event-return",
        ToParent { .. } => "satellite-event-to-parent",
        Definitive { .. } => "satellite-event-definitive",
        YouthPull { .. } => "satellite-event-youth-pull",
        SellOn { .. } => "satellite-event-sell-on",
        AiAcquisition { .. } => "satellite-event-ai-acquisition",
        AiPartnership { .. } => "satellite-event-ai-partnership",
        AiLoan { .. } => "satellite-event-ai-loan",
        AiPromotion { .. } => "satellite-event-ai-promotion",
        AiCession { .. } => "satellite-event-ai-cession",
    }
}

/// `SuperMundial.Vaga.via`: how a club got its place.
pub fn super_mundial_route(t: &Translator, route: &brasfoot_model::mods::SuperMundialRoute) -> String {
    use brasfoot_model::mods::SuperMundialRoute::*;
    match route {
        Champion { competition: c, year } => {
            t.text("super-mundial-route-champion", Some(&args([("competition", competition(t, c)), ("year", year.to_string())])))
        }
        Ranking => t.text("super-mundial-route-ranking", None),
        Host { country: k } => t.text("super-mundial-route-host", Some(&args([("country", country(t, *k))]))),
        Reallocated => t.text("super-mundial-route-reallocated", None),
    }
}

/// The player of `SuperMundial.rotulo`: his name and his club then.
fn super_mundial_player(t: &Translator, a: &brasfoot_model::mods::SuperMundialAward) -> String {
    match &a.club {
        Some(club) => t.text("super-mundial-player-with-club", Some(&args([("name", a.name.clone()), ("club", club.clone())]))),
        None => t.text("super-mundial-player", Some(&args([("name", a.name.clone())]))),
    }
}

/// `Edicao.bola`: the best player with his goals and assists.
pub fn super_mundial_best_player(t: &Translator, a: &brasfoot_model::mods::SuperMundialAward) -> String {
    let player = super_mundial_player(t, a);
    t.text("super-mundial-best-player", Some(&args([("player", player), ("goals", a.goals.to_string()), ("assists", a.assists.to_string())])))
}

/// `Edicao.chuteira`: the top scorer with his goals.
pub fn super_mundial_top_scorer(t: &Translator, a: &brasfoot_model::mods::SuperMundialAward) -> String {
    let player = super_mundial_player(t, a);
    t.text("super-mundial-top-scorer", Some(&args([("player", player), ("goals", a.goals.to_string())])))
}

/// `Edicao.luva`: the goalkeeper.
pub fn super_mundial_goalkeeper(t: &Translator, a: &brasfoot_model::mods::SuperMundialAward) -> String {
    super_mundial_player(t, a)
}

/// A goal of `CampeonatoBase.JogoBase.gols`: "14' Dardo Salas (Ass: Patricio Marini) [Paris FC]".
pub fn youth_league_goal(t: &Translator, g: &brasfoot_model::mods::YouthLeagueGoal) -> String {
    let scorer = g.scorer.clone().unwrap_or_else(|| t.text("youth-league-stand-in", None));
    let detail = match g.kind {
        2 => t.text("youth-league-goal-own", None),
        3 => t.text("youth-league-goal-penalty", None),
        4 => t.text("youth-league-goal-free-kick", None),
        _ => String::new(),
    };
    let assist = g.assist.as_ref().map(|a| t.text("youth-league-goal-assist", Some(&args([("name", a.clone())])))).unwrap_or_default();
    t.text(
        "youth-league-goal",
        Some(&args([("minute", g.minute.to_string()), ("scorer", scorer), ("detail", detail), ("assist", assist), ("club", g.club.clone())])),
    )
}

/// `HistoricoCampeao.artilheiro`: "name (club, n gols)", or "-".
pub fn youth_league_top_scorer(t: &Translator, h: &brasfoot_model::mods::YouthLeagueChampion) -> String {
    match &h.top_scorer {
        Some((name, club, goals)) => t.text("youth-league-top-scorer", Some(&args([("name", name.clone()), ("club", club.clone()), ("goals", goals.to_string())]))),
        None => t.text("youth-league-nobody", None),
    }
}

/// `HistoricoCampeao.craque`: "name (club, average)", the average already formatted, or "-".
pub fn youth_league_best_player(t: &Translator, h: &brasfoot_model::mods::YouthLeagueChampion, average: &dyn Fn(f64) -> String) -> String {
    match &h.best_player {
        Some((name, club, avg)) => t.text("youth-league-best-player", Some(&args([("name", name.clone()), ("club", club.clone()), ("average", average(*avg))]))),
        None => t.text("youth-league-nobody", None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brasfoot_model::labels::StageDivision;

    #[test]
    fn national_league_names_are_composed_as_the_original_does() {
        let t = Translator::new(crate::DEFAULT_LOCALE);
        assert_eq!(national_league(&t, 29), "Campeonato Brasileiro");
        let league = |country, division| StageLabel { name: String::new(), division, league_of: Some(country) };
        assert_eq!(stage(&t, &league(29, StageDivision::Named("Série A".into()))), " Brasileiro - Série A");
        assert_eq!(stage(&t, &league(3, StageDivision::None)), " Alemão");
        // ad.br(68) is "Campeonato dos EUA" and getNome() cuts "Campeonato do": the original shows "s EUA"
        assert_eq!(stage(&t, &league(68, StageDivision::Number(2))), "s EUA - 2ª D");
        // a name from the league's configuration wins
        let named = StageLabel { name: " Liga Exemplo".into(), division: StageDivision::None, league_of: None };
        assert_eq!(stage(&t, &named), " Liga Exemplo");
    }

    #[test]
    fn national_cups_carry_the_article_of_the_country() {
        let t = Translator::new(crate::DEFAULT_LOCALE);
        assert_eq!(competition(&t, &CompetitionLabel::Cup { country: 29 }), "Copa do Brasil");
        // no article of its own: "da"
        assert_eq!(competition(&t, &CompetitionLabel::Cup { country: 31 }), "Copa da Bulgária");
    }
}
