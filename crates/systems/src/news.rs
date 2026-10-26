// Port of what bfmod.Jornal and bfmod.Mensal do on the mod's clock besides their texts (the
// newspaper's news are not ported): Jornal.tick brings the Hall of Fame and the gems (Joias) up
// to date at each new season; Mensal.tick pays the human club's monthly items of the systems.

use std::collections::{HashMap, HashSet};

use brasfoot_model::ids::PlayerId;
use brasfoot_model::mods::Gem;
use brasfoot_model::rng::java_round;

use crate::Mod;
use crate::awards::season::player_key;

/// `Jornal.TIPO_NOTICIA`: the sender of the mod's news in the coaches' messages.
const NEWS_SENDER: i32 = 77;
/// `Joias.POR_TEMPORADA`, `IDADE_ALVO`, `DA_BASE`
const GEMS_PER_SEASON: usize = 6;
const GEM_TARGET_AGE: i32 = 23;
const FROM_ACADEMY: i32 = -1;

impl Mod<'_> {
    /// Post a new article to Gazeta do Brasfoot (`Jornal.noticia`).
    pub fn post_article(&mut self, section: &str, title: &str, body: &str) {
        let id = self.world.mods.articles.last().map(|a| a.id + 1).unwrap_or(1);
        let date = {
            let f = self.runtime.clock.fields(self.world.today_millis());
            format!("{:02}/{:02}/{}", f.day, f.month + 1, f.year)
        };
        let article = brasfoot_model::mods::Article {
            id,
            date,
            section: section.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            read: false,
        };
        self.world.mods.articles.push(article);
        if self.world.mods.articles.len() > 800 {
            self.world.mods.articles.remove(0);
        }
    }

    /// `Jornal.tick()`: once a day with a human coach.
    pub fn news_tick(&mut self) {
        if self.world.game.human_coaches.is_empty() {
            return;
        }
        // tirarDaCaixa(): the news the mod left in the messages go to the newspaper
        for c in self.world.game.human_coaches.clone() {
            self.world.coach_mut(c).messages.retain(|m| m.sender != NEWS_SENDER);
        }
        let today = self.world.today_millis();
        let season = self.world.season();
        let Some(state) = self.world.mods.news.clone() else {
            // the first run on a save takes a baseline and announces nothing
            self.world.mods.news = Some(brasfoot_model::mods::NewsState { last_day: today, last_season: season });
            self.gems_new_season();
            return;
        };
        if state.last_day == today {
            return;
        }
        let mut state = state;
        state.last_day = today;
        if season > state.last_season {
            state.last_season = season;
            self.world.mods.news = Some(state);
            // Jornal.lendas: the news of the legends who retired is not ported
            let _retired = crate::awards::hall_of_fame::update(self.world, season - 1);
            self.gems_new_season();
        } else {
            self.world.mods.news = Some(state);
        }
        // transferencias(), marcos(): news only
    }

    /// `Joias.novaTemporada()`: the earlier gems developed, the year's best newly scouted
    /// ceilings of the academies revealed (`Base.talentosNovos`).
    fn gems_new_season(&mut self) {
        let mut active: HashMap<String, PlayerId> = HashMap::new();
        for &p in &self.world.game.players {
            active.insert(player_key(self.world, p), p);
        }
        for g in self.world.mods.gems.clone() {
            if let Some(&p) = active.get(&g.key) {
                self.develop_gem(&g, p);
            }
        }
        let year = crate::economy::year(self.world);
        if self.world.mods.gems.iter().all(|g| g.year_revealed != year) {
            let announced: HashSet<String> = self.world.mods.gems.iter().map(|g| g.key.clone()).collect();
            let mut gems_list = Vec::new();
            for (p, kid) in self.new_talents(&announced, GEMS_PER_SEASON) {
                let player = self.world.player(p);
                gems_list.push((player.name.clone(), player.age, player.position, player.club.map(|c| self.world.club(c).name.clone())));
                let gem = Gem {
                    key: player_key(self.world, p),
                    name: player.name.clone(),
                    country: player.country,
                    position: player.position,
                    club: player.club.map(|c| self.world.club(c).name.clone()),
                    year_revealed: year,
                    age_revealed: player.age,
                    initial_strength: FROM_ACADEMY,
                    target: kid.ceiling,
                    debut: 0,
                };
                self.world.mods.gems.push(gem);
            }
            if !gems_list.is_empty() {
                let mut body = String::from("<b>Olheiros do mundo inteiro apontam as maiores promessas das categorias de base:</b><br><br>");
                for (name, age, pos, club) in gems_list {
                    let pos_str = match pos { 1 => "Goleiro", 2 => "Lateral", 3 => "Zagueiro", 4 => "Volante", 5 => "Meia", _ => "Atacante" };
                    let club_str = club.as_deref().unwrap_or("-");
                    body.push_str(&format!("• {name} ({age} anos, {pos_str}), {club_str}<br>"));
                }
                self.post_article("Base", "Novas joias reveladas", &body);
            }
        }
    }

    /// `Joias.desenvolver(Joia, F)`: pulls a gem of the mod's older versions up to his point on
    /// the curve, never lowering his strength; the academy's gems are developed by `Base`.
    fn develop_gem(&mut self, g: &Gem, p: PlayerId) {
        if g.initial_strength == FROM_ACADEMY {
            return;
        }
        if self.world.player(p).potential < 10 {
            self.world.player_mut(p).set_potential(10);
        }
        let age = self.world.player(p).age;
        if age > GEM_TARGET_AGE || age <= g.age_revealed {
            return;
        }
        let progress = ((age - g.age_revealed) as f64 / (GEM_TARGET_AGE - g.age_revealed) as f64).min(1.0);
        let wanted = java_round(g.initial_strength as f64 + (g.target - g.initial_strength) as f64 * progress) as i32;
        if self.world.player(p).strength < wanted {
            self.set_strength(p, wanted, false);
        }
    }

    /// `Mensal.tick()`: each month that went by (rounds skip months), the human club's
    /// sponsorship, shares, staff, members, assets and bank.
    pub fn monthly_tick(&mut self) {
        // Diretor.meuClube(): the first human coach's club
        let Some(&coach) = self.world.game.human_coaches.first() else { return };
        if self.world.coach(coach).club_id.is_none() {
            return;
        }
        let f = self.runtime.clock.fields(self.world.today_millis());
        let month = f.year * 100 + f.month + 1;
        let Some(last) = self.world.mods.last_month_paid else {
            self.world.mods.last_month_paid = Some(month);
            return;
        };
        if month == last {
            return;
        }
        let count = |m: i32| m / 100 * 12 + m % 100 - 1;
        let months = (count(month) - count(last)).clamp(1, 12);
        self.world.mods.last_month_paid = Some(month);
        for _ in 0..months {
            self.pay_master_sponsor_month();
            self.pay_sponsor_slots_month();
            self.pay_staff_month();
            self.pay_fans_month();
            self.pay_assets_month();
            self.pay_bank_month();
        }
        // Emprestimos.relatorioMensal(): the report on the loanees is news only
    }
}
