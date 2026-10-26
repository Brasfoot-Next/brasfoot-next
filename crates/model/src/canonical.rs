//! A canonical text of a [`World`]: every struct reachable from the game's roots, fields in order,
//! shared objects numbered in first-visit order (like the Kryo stream) so two worlds compare
//! equal whatever the order of their arenas. The differential tests compare a simulated world
//! with the world the game itself saved.

use std::collections::HashMap;
use std::fmt::Write;

use crate::ids::*;
use crate::world::World;

pub struct Canonical<'w> {
    world: &'w World,
    out: String,
    seen: HashMap<(u8, u32), u32>,
    next: u32,
}

pub trait Visit {
    fn visit(&self, c: &mut Canonical);
}

impl<'w> Canonical<'w> {
    pub fn line(&mut self, text: &str) {
        self.out.push_str(text);
        self.out.push('\n');
    }

    pub fn field(&mut self, name: &str) {
        self.out.push('.');
        self.out.push_str(name);
        self.out.push('\n');
    }

    /// Visits an arena object the first time, refers back to it afterwards.
    fn shared(&mut self, arena: u8, index: u32, visit: impl FnOnce(&mut Self)) {
        if let Some(n) = self.seen.get(&(arena, index)) {
            let n = *n;
            let _ = writeln!(self.out, "^{n}");
            return;
        }
        self.next += 1;
        let n = self.next;
        self.seen.insert((arena, index), n);
        let _ = writeln!(self.out, "{{{n}");
        visit(self);
    }
}

/// The canonical text of a world, from its two roots.
pub fn canonical(world: &World) -> String {
    let mut c = Canonical { world, out: String::new(), seen: HashMap::new(), next: 0 };
    c.line("game");
    world.game.visit(&mut c);
    c.line("job_market");
    world.job_market.visit(&mut c);
    c.out
}

macro_rules! visit_arena {
    ($($id:ident, $arena:literal, $field:ident);* $(;)?) => {$(
        impl Visit for $id {
            fn visit(&self, c: &mut Canonical) {
                let world = c.world;
                c.shared($arena, self.0, |c| world.$field[self.index()].visit(c));
            }
        }
    )*};
}

visit_arena!(
    PlayerId, 0, players;
    ClubId, 1, clubs;
    CoachId, 2, coaches;
    StadiumId, 3, stadiums;
    MatchId, 4, matches;
    CompetitionId, 5, competitions;
    StageId, 6, stages;
    CountryId, 7, countries;
    StateLeagueId, 8, state_leagues;
);

macro_rules! visit_display {
    ($($t:ty),*) => {$(
        impl Visit for $t {
            fn visit(&self, c: &mut Canonical) {
                c.line(&self.to_string());
            }
        }
    )*};
}

visit_display!(i8, i16, u16, i32, i64, bool);

impl Visit for f64 {
    fn visit(&self, c: &mut Canonical) {
        c.line(&format!("d{:x}", self.to_bits()));
    }
}

impl Visit for f32 {
    fn visit(&self, c: &mut Canonical) {
        c.line(&format!("f{:x}", self.to_bits()));
    }
}

impl Visit for String {
    fn visit(&self, c: &mut Canonical) {
        c.line(&format!("{self:?}"));
    }
}

impl<T: Visit> Visit for Option<T> {
    fn visit(&self, c: &mut Canonical) {
        match self {
            Some(v) => v.visit(c),
            None => c.line("N"),
        }
    }
}

impl<T: Visit> Visit for Vec<T> {
    fn visit(&self, c: &mut Canonical) {
        c.line(&format!("[{}", self.len()));
        for v in self {
            v.visit(c);
        }
    }
}

impl<T: Visit, const N: usize> Visit for [T; N] {
    fn visit(&self, c: &mut Canonical) {
        for v in self {
            v.visit(c);
        }
    }
}
