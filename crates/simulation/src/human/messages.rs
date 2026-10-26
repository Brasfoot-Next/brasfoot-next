// Port of the message center a.bn: a message read (a(as)), and the answer to the board's
// question about fining a player (cQ(int), nQ()).

use brasfoot_engine::messages;
use brasfoot_model::ids::{ClubId, CoachId};

use crate::Sim;

impl Sim<'_> {
    /// `bn.a(as)`: the message shown is read (`as.aF(true)`).
    pub fn read_message(&mut self, coach: CoachId, index: usize) {
        if let Some(m) = self.world.coach_mut(coach).messages.get_mut(index) {
            m.read = true;
        }
    }

    /// `bn.cQ(1)` and `cQ(2)`: the board's question is answered (`as.aG(false)`). Fining
    /// takes the tenth of the salary the message keeps (`ah.v(oq, 9)`, "Multas/outros") and,
    /// one time in two or so, the player complains (`nQ()`: a new message of kind 10, one of
    /// the texts 16 to 19). True when the player was fined.
    pub fn answer_fine(&mut self, club: ClubId, coach: CoachId, index: usize, fine: bool) -> bool {
        let Some(m) = self.world.coach_mut(coach).messages.get_mut(index) else { return false };
        m.flag_rz = false;
        if !fine {
            return false;
        }
        let (kind, amount, player) = (m.value_sc, m.amount, m.text3.clone());
        if kind != 1 || amount <= 0 {
            return false;
        }
        self.world.club_income(club, amount, 9);
        // nQ(): new Random().nextInt(100) > 50, then new Random().nextInt(4) + 16
        if self.runtime.rng.fresh_int(100) > 50 {
            let text = self.runtime.rng.fresh_int(4) + 16;
            messages::send(self.world, coach, 10, text, player, String::new());
        }
        true
    }
}
