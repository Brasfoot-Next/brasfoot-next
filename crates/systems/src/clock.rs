// Port of bfmod.BolaDeOuro.tick: the mod's clock. The mod has no thread; the game's player card
// (components.m) calls it every time the main screen is built, and every periodic system hangs
// off it, in this order.

use crate::economy::humans;
use crate::Mod;

impl Mod<'_> {
    /// `BolaDeOuro.tick(Object)` without its window parts (`Diagnostico`, `Menu.instalar`).
    pub fn main_screen_tick(&mut self) {
        let human = !humans(self.world).is_empty();
        // Taticas.prepararProximo: the automatic tactics of the player's next match
        if human {
            self.prepare_next_tactic();
        }
        self.runtime.checkpoint("mod Estadios");
        self.stadiums_tick();
        self.runtime.checkpoint("mod Base");
        self.academy_tick();
        self.runtime.checkpoint("mod Jornal");
        self.news_tick();
        self.runtime.checkpoint("mod Economia");
        self.economy_tick();
        self.runtime.checkpoint("mod Olheiros");
        self.scouts_tick();
        self.runtime.checkpoint("mod Torcida");
        self.fans_tick();
        self.runtime.checkpoint("mod Mensal");
        self.monthly_tick();
        self.runtime.checkpoint("mod Patrocinio");
        self.master_sponsor_tick();
        self.runtime.checkpoint("mod Investidor");
        self.investor_tick();
        self.runtime.checkpoint("mod Cotas");
        self.sponsor_slots_tick();
        self.runtime.checkpoint("mod Caixa");
        self.cash_tick();
        self.runtime.checkpoint("mod Emprestimos");
        self.loans_tick();
        self.runtime.checkpoint("mod SuperMundial");
        self.super_mundial_tick();
        self.runtime.checkpoint("mod end");
    }
}
