//! The numbers the original gives the players when it saves, by which the transfer history finds
//! its players again when it loads (`best.ap.U` is transient). Port of `g.b.Ai()` (its players'
//! part), `g.b.Aj()`, `ap.mb()` and `ap.me()`.

use std::collections::HashMap;

use crate::ids::PlayerId;
use crate::world::World;

/// `F.ej` of a player of `c.a.SR.O()`.
const MAIN_LIST: i32 = 1;
/// `F.ej` of a player of `c.a.SR.Q()`.
const YOUTH_LIST: i32 = 2;

impl World {
    /// Port of `g.b.Ai()`: before a save the players of `c.a.SR.O()` then `c.a.SR.Q()` get one
    /// running number and the list they are in, and each transfer records the number of its
    /// player (`ap.mb()`). A player who is in neither list (retired) keeps the number of the
    /// previous save, or -1 if he was created after it, so after a load his transfers show
    /// whoever holds that number now. That is the original's behaviour and is kept.
    pub fn number_players_for_save(&mut self) {
        let mut number = 0;
        for (list, players) in [(MAIN_LIST, &self.game.players), (YOUTH_LIST, &self.game.youth_players)] {
            for &p in players {
                let player = &mut self.players[p.index()];
                player.save_number = number;
                player.save_list = list;
                number += 1;
            }
        }
        for t in &mut self.game.transfers {
            match t.player {
                Some(p) => {
                    t.player_number = self.players[p.index()].save_number;
                    t.player_list = self.players[p.index()].save_list;
                }
                None => t.player_number = -1,
            }
        }
    }

    /// Port of `g.b.Aj()`: after a load each transfer finds its player again (`ap.me()` on a
    /// record whose transient player is null).
    pub fn relink_transfers(&mut self) {
        let numbers = self.save_numbers();
        for t in &mut self.game.transfers {
            t.player = None;
            if let Some(&p) = numbers.get(&(t.player_list, t.player_number)) {
                t.player = Some(p);
            }
        }
    }

    /// Port of `ap.me()` on one transfer, as the mod calls it on a record without a player.
    pub fn relink_transfer(&mut self, transfer: usize) {
        let t = &self.game.transfers[transfer];
        let (list, number) = (t.player_list, t.player_number);
        if number < 0 {
            return;
        }
        let players = match list {
            MAIN_LIST => &self.game.players,
            YOUTH_LIST => &self.game.youth_players,
            _ => return,
        };
        if let Some(&p) = players.iter().find(|&&p| self.players[p.index()].save_number == number) {
            self.game.transfers[transfer].player = Some(p);
        }
    }

    /// `(list, number)` to the first player of that list with that number, as `ap.me()` finds him.
    fn save_numbers(&self) -> HashMap<(i32, i32), PlayerId> {
        let mut numbers = HashMap::new();
        for (list, players) in [(MAIN_LIST, &self.game.players), (YOUTH_LIST, &self.game.youth_players)] {
            for &p in players {
                let number = self.players[p.index()].save_number;
                if number >= 0 {
                    numbers.entry((list, number)).or_insert(p);
                }
            }
        }
        numbers
    }
}

#[cfg(test)]
mod tests {
    use crate::ids::PlayerId;
    use crate::saved::{Player, Transfer};
    use crate::world::World;

    fn player(name: &str) -> Player {
        Player { name: name.to_owned(), ..Default::default() }
    }

    /// A retired player keeps his old number; after the save and load his transfer shows the
    /// player who holds that number now, as in the original.
    #[test]
    fn a_retired_players_transfer_shows_who_took_his_number() {
        let mut world = World::default();
        world.players = vec![player("Fulano Inventado"), player("Beltrano de Teste"), player("Ciclano Fictício")];
        world.game.players = vec![PlayerId(0), PlayerId(1)];
        world.game.youth_players = vec![PlayerId(2)];
        world.game.transfers = vec![Transfer { player: Some(PlayerId(1)), fee: 1000, ..Default::default() }];
        world.number_players_for_save();
        assert_eq!((world.players[2].save_number, world.players[2].save_list), (2, 2));
        world.relink_transfers();
        assert_eq!(world.game.transfers[0].player, Some(PlayerId(1)));

        // Beltrano retires; a new player arrives in front of the list
        world.players.push(player("Novato Imaginário"));
        world.game.players = vec![PlayerId(3), PlayerId(0)];
        world.game.transfers.push(Transfer { player: Some(PlayerId(3)), ..Default::default() });
        world.number_players_for_save();
        assert_eq!(world.game.transfers[0].player_number, 1);
        world.relink_transfers();
        // Fulano is number 1 now
        assert_eq!(world.game.transfers[0].player, Some(PlayerId(0)));
        assert_eq!(world.game.transfers[1].player, Some(PlayerId(3)));
    }

    /// A player created after the last save and gone before the next one has no number.
    #[test]
    fn a_player_never_saved_leaves_his_transfer_empty() {
        let mut world = World::default();
        world.players = vec![player("Fulano Inventado"), player("Passageiro de Teste")];
        world.game.players = vec![PlayerId(0)];
        world.game.transfers = vec![Transfer { player: Some(PlayerId(1)), ..Default::default() }];
        world.number_players_for_save();
        world.relink_transfers();
        assert_eq!(world.game.transfers[0].player, None);
    }
}

impl World {
    /// The calendar's competitions in their order of first appearance (`g.b.Ag`, `g.b.Af`).
    fn calendar_competitions(&self) -> Vec<crate::ids::CompetitionId> {
        let mut order = Vec::new();
        for day in &self.game.days {
            for &c in &day.competitions {
                if !order.contains(&c) {
                    order.push(c);
                }
            }
        }
        order
    }

    /// Port of `g.b.Ag()` before a save, then of the load (`g.b.Af()`): the calendar's
    /// competitions get their order of first appearance as id (`at.az(int)`); each record of a
    /// player of `c.a.SR.O()` takes its competition's id when it has one (`H.gY()`), else keeps
    /// the number of an older save. A competition the calendar no longer has keeps its old id
    /// and lends it to its records; one made since the last save has none. The links are
    /// transient and the load makes them again.
    pub fn number_competitions_for_save(&mut self) {
        let order = self.calendar_competitions();
        for (i, &c) in order.iter().enumerate() {
            self.competition_mut(c).legacy_id = i as i32;
        }
        let ids: Vec<i32> = self.competitions.iter().map(|c| c.legacy_id).collect();
        for p in self.game.players.clone() {
            for h in &mut self.players[p.index()].competition_stats {
                if let Some(c) = h.competition_id
                    && ids[c.index()] >= 0
                {
                    h.competition_number = ids[c.index()];
                }
            }
        }
        for player in &mut self.players {
            for h in &mut player.competition_stats {
                h.competition_id = None;
            }
        }
        self.link_competition_stats();
    }

    /// Port of `g.b.Af()` on load: each record of a player of `c.a.SR.O()` with a number links
    /// to the calendar's competition of that number, when there is one; the others stay
    /// unlinked.
    pub fn link_competition_stats(&mut self) {
        let order = self.calendar_competitions();
        let by_number: HashMap<i32, crate::ids::CompetitionId> = order.iter().map(|&c| (self.competition(c).legacy_id, c)).rev().collect();
        for p in self.game.players.clone() {
            for h in &mut self.players[p.index()].competition_stats {
                if h.competition_number >= 0
                    && let Some(&c) = by_number.get(&h.competition_number)
                {
                    h.competition_id = Some(c);
                }
            }
        }
    }
}
