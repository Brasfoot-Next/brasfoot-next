// Port of bfmod.Caixa: the history of the player's club finances for the finances window. The
// game only keeps the running totals of the current season (best.C, reset at the season-end
// screen), so each tick stores the latest totals of the month and the cash of the day; monthly
// flows come from the difference between consecutive months.

use brasfoot_model::clock::Clock;
use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::CashMonth;
use brasfoot_model::world::World;

use crate::Mod;
use crate::staff::my_club;

/// `RECEITAS`, `DESPESAS`: how many lines of each (their names are in the catalog).
pub const INCOME_LINES: usize = 5;
pub const EXPENSE_LINES: usize = 6;
/// "Multas e outros": the game never resets it, so it always counts by difference.
const OTHER: usize = 4;
const MAX_DAYS: usize = 400;

/// `Caixa.Fluxo`: money in and out during one month (or since the first record, when partial).
#[derive(Clone, Debug)]
pub struct Flow {
    pub month: i32,
    pub season: i32,
    pub partial: bool,
    pub values: Vec<i64>,
}

impl Flow {
    pub fn income(&self) -> i64 {
        self.values[..INCOME_LINES].iter().sum()
    }

    pub fn expenses(&self) -> i64 {
        self.values[INCOME_LINES..].iter().sum()
    }
}

/// `Caixa.totais(ah)`: the game's running totals, the income lines then the expense lines.
pub fn totals(world: &World, club: ClubId) -> Vec<i64> {
    let f = &world.club(club).finances;
    vec![
        f.income_type5 as i64,
        f.income_transfers,
        f.income_type3 as i64,
        f.income_type6 as i64,
        f.income_type9 as i64,
        f.expense_transfers,
        f.expense_type7 as i64,
        f.expense_salaries,
        f.expense_type4 as i64,
        f.expense_type8 as i64,
        f.expense_other as i64,
    ]
}

/// `Caixa.hoje()`: today as `yyyymmdd`.
pub fn today(world: &World, clock: &Clock) -> i32 {
    date(clock, world.today_millis())
}

/// `Caixa.data(Calendar)`
pub fn date(clock: &Clock, millis: i64) -> i32 {
    let f = clock.fields(millis);
    f.year * 10000 + (f.month + 1) * 100 + f.day
}

/// `Caixa.fluxos()` without its tick: the current club's monthly flows, oldest first.
pub fn flows(world: &World) -> Vec<Flow> {
    let mut out = Vec::new();
    let Some(mine) = my_club(world) else { return out };
    let id = world.club(mine).id;
    let mut before: Option<&CashMonth> = None;
    for m in world.mods.cash_book.months.values() {
        if m.club != id {
            before = None;
            continue;
        }
        let mut values = vec![0; m.values.len()];
        let partial = before.is_none();
        match before {
            // the first month on record: what the season had so far (the "other" line is lifelong)
            None => {
                values.copy_from_slice(&m.values);
                values[OTHER] = 0;
            }
            Some(b) => {
                let same = b.season == m.season;
                for i in 0..values.len() {
                    values[i] = if same || i == OTHER { m.values[i] - b.values.get(i).copied().unwrap_or(0) } else { m.values[i] };
                }
            }
        }
        out.push(Flow { month: m.month, season: m.season, partial, values });
        before = Some(m);
    }
    out
}

/// `Caixa.totaisDaTemporada(int)`: the current season from the game, "other" from the months.
pub fn season_totals(world: &World, season: i32) -> Vec<i64> {
    let Some(mine) = my_club(world) else { return vec![0; INCOME_LINES + EXPENSE_LINES] };
    let list = flows(world);
    if season == world.season() {
        let mut out = totals(world, mine);
        out[OTHER] = list.iter().filter(|f| f.season == season).map(|f| f.values[OTHER]).sum();
        return out;
    }
    let mut out = vec![0; INCOME_LINES + EXPENSE_LINES];
    for f in list.iter().filter(|f| f.season == season) {
        for (o, v) in out.iter_mut().zip(&f.values) {
            *o += v;
        }
    }
    out
}

/// `Caixa.temTemporada(int)`
pub fn has_season(world: &World, season: i32) -> bool {
    flows(world).iter().any(|f| f.season == season)
}

/// `Caixa.mesesDecorridos()`: the months of the season gone by, today's fraction counted, at
/// least half a month.
pub fn months_elapsed(world: &World, clock: &Clock) -> f64 {
    let millis = world.today_millis();
    let f = clock.fields(millis);
    let days_in_month = clock.days_in_month(f.year, f.month) as f64;
    (f.month as f64 + f.day as f64 / days_in_month).max(0.5)
}

/// `Caixa.folhaMensal(ah)`: the game can be set to weekly wages.
pub fn monthly_wages(world: &World, club: ClubId) -> i64 {
    let wages = world.club_wages(club);
    if world.game.monthly_salary { wages } else { brasfoot_model::rng::java_round(wages as f64 * 52.0 / 12.0) }
}

impl Mod<'_> {
    /// `Caixa.tick()`: the month's totals and the day's cash of the player's club.
    pub fn cash_tick(&mut self) {
        let Some(mine) = my_club(self.world) else { return };
        let today = today(self.world, &self.runtime.clock);
        let month = CashMonth { month: today / 100, season: self.world.season(), club: self.world.club(mine).id, cash: self.world.club(mine).cash, values: totals(self.world, mine) };
        let cash = self.world.club(mine).cash;
        let book = &mut self.world.mods.cash_book;
        book.months.insert(month.month, month);
        book.days.insert(today, cash);
        while book.days.len() > MAX_DAYS {
            let first = *book.days.keys().next().expect("not empty");
            book.days.remove(&first);
        }
    }
}
