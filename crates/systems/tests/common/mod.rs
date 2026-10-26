//! What the differential tests of the mod systems share: the mod's side files written from the
//! Rust state, in the original's format.

#![allow(dead_code)] // each test uses its own part

use brasfoot_i18n::{Translator, labels};
use brasfoot_model::brands;
use brasfoot_model::mods::{StaffMember, StaffRole};
use brasfoot_model::world::World;

/// `Economia.salvar()`, the processed transfers apart (a `HashSet`: compared sorted).
pub fn economy_file(t: &Translator, world: &World) -> (String, Vec<String>) {
    economy_file_of(t, world, world.mods.economy.as_ref().expect("the economy started"))
}

/// [`economy_file`] of an economy state (the one last saved, say).
pub fn economy_file_of(t: &Translator, world: &World, e: &brasfoot_model::mods::EconomyState) -> (String, Vec<String>) {
    let mut out = format!("TEMPORADA\t{}\n", e.last_season);
    if e.cleaned {
        out += "SANEADO\t1\n";
    }
    for c in &e.clubs {
        out += &format!(
            "CLUBE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            c.id,
            c.fortune,
            c.has_owner as i32,
            c.owner_funding,
            c.owner_since,
            c.brand.map(brands::brand).unwrap_or_default(),
            labels::sponsorship_kind(t, c.sponsorship_kind),
            c.sponsorship,
            c.sponsorship_until,
            c.project_until,
            c.owner_name.map(brands::owner).unwrap_or_default()
        );
    }
    for ev in &e.events {
        let kind = if ev.arrived { "economy-event-new-owner" } else { "economy-event-owner-left" };
        out += &format!("EVENTO\t{}\t{}\t{}\n", ev.year, ev.club, t.text(kind, None));
    }
    let mut processed: Vec<String> = e.processed.iter().cloned().collect();
    processed.sort();
    (out, processed)
}

/// `Comissao.salvar()`
pub fn staff_file(t: &Translator, world: &World) -> String {
    let s = &world.mods.staff;
    if s.hired.is_empty() && s.candidates.is_empty() && s.candidates_month == 0 {
        return String::new();
    }
    let role = |r: StaffRole| match r {
        StaffRole::Analyst => "ANALISTA",
        StaffRole::Assistant => "AUXILIAR",
        StaffRole::FitnessCoach => "PREPARADOR",
    };
    let line = |tag: &str, m: &StaffMember| {
        format!("{tag}\t{}\t{}\t{}\t{}\t{}\t{}\t-\n", role(m.role), brands::staff(m.name), m.level, m.salary, labels::staff_profile(t, m.profile), m.since)
    };
    let mut out = format!("MES\t{}\n", s.candidates_month);
    for m in &s.hired {
        out += &line("STAFF", m);
    }
    for m in &s.candidates {
        out += &line("CANDIDATO", m);
    }
    out
}

/// `Double.toString(double)`: the shortest decimal that reads back as the same double, with a
/// fraction digit always, in scientific notation outside [10^-3, 10^7).
pub fn java_double(v: f64) -> String {
    let magnitude = v.abs();
    if v == 0.0 || (1e-3..1e7).contains(&magnitude) {
        let s = format!("{v}");
        return if s.contains('.') { s } else { format!("{s}.0") };
    }
    let s = format!("{v:e}");
    let (mantissa, exponent) = s.split_once('e').unwrap();
    let mantissa = if mantissa.contains('.') { mantissa.to_string() } else { format!("{mantissa}.0") };
    format!("{mantissa}E{exponent}")
}

/// `Base.salvar()`
pub fn academy_file(world: &World) -> String {
    let mut out = String::new();
    for k in world.mods.academy.iter().flatten() {
        out += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            k.key.replace(['\t', '\n'], " "),
            k.ceiling,
            java_double(k.development_age),
            java_double(k.scout_error),
            k.professional as i32,
            k.notified as i32,
            k.year_joined,
            java_double(k.last_time)
        );
    }
    out
}

/// `Joias.escrever()`
pub fn gems_file(world: &World) -> String {
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    let mut out = String::new();
    for g in &world.mods.gems {
        out += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            clean(&g.key),
            clean(&g.name),
            g.country,
            g.position,
            clean(g.club.as_deref().unwrap_or("-")),
            g.year_revealed,
            g.age_revealed,
            g.initial_strength,
            g.target,
            g.debut
        );
    }
    out
}

/// `Torcida.salvar()`; the dates as `best.a.f()` writes them.
pub fn fans_file(t: &Translator, world: &World, clock: &brasfoot_model::clock::Clock) -> String {
    let f = &world.mods.fans;
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    let mut out = String::new();
    for (key, strength) in &f.squad {
        out += &format!("ELENCO\t{}\t{strength}\n", clean(key));
    }
    for key in &f.academy {
        out += &format!("BASE\t{}\n", clean(key));
    }
    for r in &f.reasons {
        let d = clock.fields(r.day);
        let date = format!("{:02}/{:02}/{:04}", d.day, d.month + 1, d.year);
        let change = if r.change > 0 { format!("+{}", r.change) } else { r.change.to_string() };
        out += &format!("MOTIVO\t{date}\t{change}\t{}\n", clean(&labels::fan_event(t, &r.why)));
    }
    out
}

/// `String.format("%.1f", x)` in pt-BR: Java's `Formatter` rounds the shortest decimal of the
/// double half up, and writes a decimal comma.
pub fn java_one_decimal(x: f64) -> String {
    let s = format!("{}", x.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let mut digits: Vec<u8> = int.bytes().map(|b| b - b'0').collect();
    let first = frac.bytes().next().map_or(0, |b| b - b'0');
    digits.push(first);
    if frac.bytes().nth(1).is_some_and(|b| b >= b'5') {
        let mut i = digits.len();
        loop {
            i -= 1;
            if digits[i] == 9 {
                digits[i] = 0;
                if i == 0 {
                    digits.insert(0, 1);
                    break;
                }
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let n = digits.len();
    let int: String = digits[..n - 1].iter().map(|d| (b'0' + d) as char).collect();
    format!("{}{int},{}", if x < 0.0 && digits.iter().any(|&d| d > 0) { "-" } else { "" }, digits[n - 1])
}

/// `Patrocinio.mi(long)`
pub fn java_millions(v: i64) -> String {
    format!("{} mi", java_one_decimal(v as f64 / 1e6))
}

/// `Patrocinio.salvar()`
pub fn master_sponsor_file(t: &Translator, world: &World) -> String {
    use brasfoot_model::mods::MasterKind;
    let s = &world.mods.master_sponsor;
    let mut out = String::new();
    if let Some(k) = &s.contract {
        let kind = match k.kind {
            MasterKind::Fixed => "Fixo",
            MasterKind::Mixed => "Misto",
            MasterKind::Risky => "Arriscado",
        };
        out += &format!(
            "CONTRATO\t{}\t{kind}\t{}\t{}\t{}\t{}\t{}\n",
            brands::MASTER_BRANDS[k.brand as usize],
            k.yearly,
            k.bonus_top4,
            k.bonus_league,
            k.bonus_continental,
            k.years
        );
    }
    for h in &s.history {
        out += &format!("HIST\t{}\n", labels::master_note(t, h, &java_millions));
    }
    out
}

/// `Cotas.salvar()`; nothing before the first save (the first tick with a club).
pub fn sponsor_slots_file(t: &Translator, world: &World) -> String {
    use brasfoot_model::mods::{SlotContract, SlotKind, SponsorSlot};
    let s = &world.mods.sponsor_slots;
    if s.club < 0 {
        return String::new();
    }
    let name = |slot: SponsorSlot| match slot {
        SponsorSlot::Sleeve => "MANGA",
        SponsorSlot::Back => "COSTAS",
        SponsorSlot::Shorts => "CALCAO",
        SponsorSlot::Kit => "MATERIAL",
        SponsorSlot::Stadium => "ESTADIO",
        SponsorSlot::Partner => "PARCEIRO",
    };
    let line = |tag: &str, k: &SlotContract| {
        let kind = match k.kind {
            SlotKind::Fixed => "Fixo",
            SlotKind::Short => "Curto",
            SlotKind::WithBonus => "Com bônus",
        };
        format!("{tag}\t{}\t{}\t{kind}\t{}\t{}\t{}\t{}\n", name(k.slot), brands::slot_brand(k.brand), k.yearly, k.bonus, k.years, k.valid_until)
    };
    let mut out = format!("CLUBE\t{}\n", s.club);
    for k in &s.deals {
        out += &line("ATIVO", k);
    }
    for k in &s.offers {
        out += &line("OFERTA", k);
    }
    for (slot, day) in &s.searches {
        out += &format!("BUSCA\t{}\t{day}\n", name(*slot));
    }
    for h in &s.history {
        out += &format!("HIST\t{}\n", labels::slot_note(t, h, &java_millions));
    }
    out
}

/// `Investidor.salvar()`; nothing before the first tick.
pub fn investor_file(world: &World) -> String {
    let s = &world.mods.investor;
    if *s == brasfoot_model::mods::InvestorState::default() {
        return String::new();
    }
    let mut out = format!("SORTEIO\t{}\nPAGO\t{}\nAVALIADO\t{}\nCOBRANCA\t{}\t{}\n", s.year_drawn, s.year_paid, s.year_judged, s.club, s.complaints);
    if let Some(o) = &s.offer {
        out += &format!("OFERTA\t{}\t{}\t{}\t{}\n", o.day, brands::owner(o.investor), o.now, o.yearly);
    }
    out
}

/// `Patrimonio.salvar()`; nothing before it starts.
pub fn assets_file(world: &World) -> String {
    let Some(a) = &world.mods.assets else { return String::new() };
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    let mut out = format!("TEMPORADA\t{}\n", a.last_season);
    for e in &a.clubs {
        out += &format!(
            "CLUBE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            e.id,
            e.arena,
            e.training_centre,
            e.academy,
            e.marketing,
            clean(&e.stadium_base_name),
            clean(&e.naming_brand),
            e.naming_yearly,
            e.naming_until,
            e.campaign_season,
            e.campaign_kind,
            e.last_trials
        );
    }
    out
}

/// `Estadios.salvar()`
pub fn stadiums_file(world: &World) -> String {
    let s = &world.mods.stadiums;
    let clean = |t: &str| t.replace(['\t', '\n'], " ");
    let mut out = String::new();
    for r in &s.renamed {
        out += &format!("RENOMEADO\t{}\t{}\t{}\t{}\t{}\n", r.club, r.year, clean(&r.old), clean(&r.new), clean(&r.legend));
    }
    if s.never {
        out += "NUNCA\n";
    }
    if s.pending {
        out += "PENDENTE\n";
    }
    out
}

/// `Satelites.salvar()`: the networks as the mod writes them; the history's detail column, worded
/// by the mod, is left out (and so on the Java side by the caller).
pub fn satellites_file(t: &Translator, world: &World) -> String {
    use brasfoot_model::mods::{LoanRole, SatellitePolicy};
    let Some(s) = &world.mods.satellites else { return String::new() };
    let clean = |t: &str| t.replace(['\t', '\n'], " ");
    let policy = |p: SatellitePolicy| match p {
        SatellitePolicy::YoungPlayers => "FORMAR_JOVENS",
        SatellitePolicy::Promotion => "BUSCAR_ACESSO",
        SatellitePolicy::Profit => "GERAR_LUCRO",
    };
    let role = |r: LoanRole| match r {
        LoanRole::Starter => "TITULAR_ABSOLUTO",
        LoanRole::Regular => "TITULAR_REGULAR",
        LoanRole::Reserve => "RESERVA",
    };
    let mut out = format!("TEMPORADA\t{}\nINICIADO\t{}\n", s.last_season, if s.ai_started { 1 } else { 0 });
    for v in &s.links {
        out += &format!("VINCULO\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n", v.satellite, v.parent, if v.owned { 1 } else { 0 }, policy(v.policy), v.yearly_fee, v.until, v.since, v.price_paid);
    }
    for (id, ts) in &s.tactics {
        out += &format!("TATICA\t{}\t{}\t{}\t{}\t{}\n", id, if ts.mirror { 1 } else { 0 }, ts.style, ts.marking, ts.side);
    }
    for (k, r) in &s.roles {
        out += &format!("PAPEL\t{}\t{}\n", clean(k), role(*r));
    }
    for h in s.history.iter().skip(s.history.len().saturating_sub(120)) {
        let kind = t.text(brasfoot_i18n::labels::satellite_event_key(&h.event), None);
        out += &format!("HIST\t{}\t{}\t{}\t{}\n", h.year, clean(&kind), clean(h.parent.as_deref().unwrap_or("-")), clean(h.satellite.as_deref().unwrap_or("-")));
    }
    out
}

/// The `.satelites` the mod wrote, without the history's detail column.
pub fn satellites_java(text: &str) -> String {
    text.lines()
        .map(|l| if l.starts_with("HIST\t") { l.split('\t').take(5).collect::<Vec<_>>().join("\t") } else { l.to_owned() })
        .map(|l| l + "\n")
        .collect()
}

/// `Credito.salvar()`; the statuses come from a `HashMap` and are compared sorted.
pub fn credit_file(world: &World) -> String {
    use brasfoot_model::mods::CreditStage;
    let Some(b) = &world.mods.credit else { return String::new() };
    let mut out = format!("PROXIMO_ID\t{}\n", b.next_id);
    for (_, loans) in &b.loans {
        for e in loans {
            out += &format!(
                "EMPRESTIMO\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                e.club, e.id, e.original, e.balance, e.instalments, e.paid, e.instalment, java_double(e.monthly_rate), e.months_late, e.year, e.month
            );
        }
    }
    let mut statuses: Vec<String> = b
        .statuses
        .iter()
        .map(|s| {
            let stage = match s.stage {
                CreditStage::Normal => "NORMAL",
                CreditStage::Warning => "ADVERTENCIA",
                CreditStage::TransferBan => "TRANSFER_BAN",
                CreditStage::Intervention => "INTERVENCAO",
            };
            format!("PUNICAO\t{}\t{}\t{}\t{}\t{}\n", s.club, stage, s.points_deducted, s.last_deduction_year, s.withheld)
        })
        .collect();
    statuses.sort();
    out + &statuses.concat()
}

/// The `.credito` the mod wrote, its statuses sorted.
pub fn credit_java(text: &str) -> String {
    let mut head = String::new();
    let mut statuses = Vec::new();
    for l in text.lines() {
        if l.starts_with("PUNICAO\t") { statuses.push(format!("{l}\n")) } else { head += &format!("{l}\n") }
    }
    statuses.sort();
    head + &statuses.concat()
}

/// `Emprestimos.salvar()`
pub fn loans_file(world: &World) -> String {
    match &world.mods.loans {
        Some(l) => format!("DIA\t{}\t{}\n", l.season_seen, l.day_seen),
        None => String::new(),
    }
}

/// `SuperMundial.salvar()`, its lines sorted (the original walks `HashMap`s).
pub fn super_mundial_file(t: &Translator, world: &World) -> Vec<String> {
    use brasfoot_i18n::labels;
    let s = &world.mods.super_mundial;
    let clean = |t: &str| t.replace(['\t', '\n'], " ");
    let mut out = Vec::new();
    if s.renamed {
        out.push("RENOMEADO\t1".to_owned());
    }
    for (year, m) in &s.coefficients {
        out.push(format!("ANO_COEF\t{year}"));
        for (club, v) in m {
            out.push(format!("COEF\t{year}\t{club}\t{}", java_double(*v)));
        }
    }
    for (year, (country, city)) in &s.hosts {
        out.push(format!("SEDE\t{year}\t{country}\t{}", clean(city)));
    }
    for (year, slots) in &s.qualified {
        for v in slots {
            let route = labels::super_mundial_route(t, &v.route);
            out.push(format!("VAGA\t{year}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", v.club, clean(&v.name), v.country, v.confederation, clean(&route), java_double(v.coefficient), v.group));
        }
    }
    for (year, days) in &s.days {
        out.push(format!("DIAS\t{year}{}", days.iter().map(|d| format!("\t{d}")).collect::<String>()));
    }
    for (year, days) in &s.paid {
        for d in days {
            out.push(format!("PAGO\t{year}\t{d}"));
        }
    }
    for e in s.editions.values() {
        let best = e.best_player.as_ref().map(|a| labels::super_mundial_best_player(t, a)).unwrap_or_default();
        let scorer = e.top_scorer.as_ref().map(|a| labels::super_mundial_top_scorer(t, a)).unwrap_or_default();
        let keeper = e.goalkeeper.as_ref().map(|a| labels::super_mundial_goalkeeper(t, a)).unwrap_or_default();
        out.push(format!(
            "EDICAO\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            e.year,
            e.host,
            clean(&e.city),
            clean(&e.champion),
            e.champion_id,
            clean(&e.runner_up),
            e.runner_up_id,
            clean(&best),
            clean(&scorer),
            clean(&keeper)
        ));
    }
    out.sort();
    out
}

/// `CampeonatoBase.salvar()`; the priority starters (a `HashSet`) sorted.
pub fn youth_league_file(t: &Translator, world: &World) -> String {
    let s = &world.mods.youth_league;
    let clean = |t: &str| t.replace(['\t', '\n'], " ");
    let mut out = format!("CFG\t{}\t{}\t{}\t{}\t{}\t{}\n", s.season, s.country, if s.finished { 1 } else { 0 }, s.style, s.marking, s.side);
    for (id, name) in &s.clubs {
        out += &format!("CLUBE\t{id}\t{}\n", clean(name));
    }
    let mut starters = s.priority_starters.clone();
    starters.sort();
    for k in starters {
        out += &format!("TITULAR\t{}\n", clean(&k));
    }
    for m in &s.matches {
        let goals: Vec<String> = m.goals.iter().map(|g| brasfoot_i18n::labels::youth_league_goal(t, g)).collect();
        out += &format!(
            "JOGO\t{}\t{}\t{:02}/{:02}/{:04}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            m.round,
            m.day,
            m.date.0,
            m.date.1,
            m.date.2,
            m.home,
            m.away,
            clean(&m.home_name),
            clean(&m.away_name),
            if m.played { 1 } else { 0 },
            m.home_goals,
            m.away_goals,
            m.home_shots,
            m.away_shots,
            m.home_possession,
            goals.join(";;").replace(['\t', '\n'], " ")
        );
    }
    for e in &s.stats {
        out += &format!(
            "STAT\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\n",
            clean(&e.key),
            clean(&e.name),
            e.country,
            e.position,
            e.age,
            e.club,
            clean(&e.club_name),
            e.games,
            e.goals,
            e.assists,
            e.rating_sum
        );
    }
    out
}

/// The `.base_camp` the mod wrote, its priority starters sorted.
pub fn youth_league_java(text: &str) -> String {
    let mut starters: Vec<&str> = text.lines().filter(|l| l.starts_with("TITULAR\t")).collect();
    starters.sort();
    let mut out = String::new();
    let mut done = false;
    for l in text.lines() {
        if l.starts_with("TITULAR\t") {
            if !done {
                for s in &starters {
                    out += s;
                    out.push('\n');
                }
                done = true;
            }
        } else {
            out += l;
            out.push('\n');
        }
    }
    out
}

/// `CampeonatoBase.salvarHistorico(List)`; the best player's average as the pt-BR harness
/// writes it (`%.2f` with a decimal comma).
pub fn youth_league_history_file(t: &Translator, world: &World) -> String {
    let clean = |t: &str| t.replace(['\t', '\n'], " ");
    let comma = |v: f64| format!("{v:.2}").replace('.', ",");
    let mut out = String::new();
    for h in &world.mods.youth_league.history {
        out += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            h.year,
            h.country,
            clean(&h.champion),
            clean(&h.runner_up),
            clean(&brasfoot_i18n::labels::youth_league_top_scorer(t, h)),
            clean(&brasfoot_i18n::labels::youth_league_best_player(t, h, &comma)),
            h.player_position
        );
    }
    out
}
