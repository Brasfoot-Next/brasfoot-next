// Port of the purchase of another club's player by a human club: the offer (best.l.a(F, ah,
// int) with c(F, ah)), the purchase of a listed player (best.l.b(F, ah) with d(F, ah)) and what
// a.cz.oL() does when the deal is done (F.a(ah, int, ...), then best.l.a(ah, F): the seller fills
// the positions it was left short of).

use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::world::World;

use crate::Sim;

/// `l.a(F, ah, int)`'s answer (0 to 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferAnswer {
    /// 0 "Oferta recusada"
    Refused,
    /// 1 "Compra realizada"
    Accepted,
    /// 2 "Jogador do seu time" (never returned by the offer, kept for its message)
    OwnPlayer,
    /// 3 "Limite de 30 jogadores alcançado" (never returned by the offer)
    SquadLimit,
    /// 4 "Não deseja se transferir para o seu time"
    Unwilling,
    /// 5 "Sem dinheiro para comprar este jogador" (never returned by the offer)
    NoMoney,
    /// 6: the club accepts, the player wants this salary to come.
    WantsSalary(i32),
    /// 7: the club asks this price, and the player this salary if any.
    Counter { price: i32, salary: Option<i32> },
}

/// `l.b(F, ah)`'s answer (0 to 5), the purchase at the listed price.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListedAnswer {
    /// 0 "Não está à venda"
    NotForSale,
    /// 1 "Compra realizada": the player has moved.
    Bought,
    /// 2 "Jogador do seu time"
    OwnPlayer,
    /// 3 "Limite de 32 jogadores alcançado" (the check is 35)
    SquadLimit,
    /// 4 "Não deseja se transferir para o seu time"
    Unwilling,
    /// 5 "Sem dinheiro para comprar este jogador"
    NoMoney,
}

/// The buying club's squad cap (`ah.kc().size() >= 35`).
const SQUAD_CAP: usize = 35;

/// `l.c(F, ah)` and `l.d(F, ah)`: 0 the player comes, 2 he comes for twice his salary (`cu`),
/// 1 he does not come. By the buyer's and the seller's reputations and confederations.
fn willingness(world: &World, p: PlayerId, buyer: ClubId) -> (i32, i32) {
    let b = world.club(buyer);
    if confederation(world, buyer) == 0 {
        return (0, 0);
    }
    let player = world.player(p);
    let seller = player.club.map(|c| world.club(c).reputation).unwrap_or(0);
    // F.gh(): the confederation of the player's club, -1 without one
    let player_confederation = player.club.map(|c| confederation(world, c)).unwrap_or(-1);
    if player_confederation == 0 && seller >= 4 {
        if b.reputation >= 4 || b.division == 1 {
            return (0, 0);
        }
        if b.reputation >= 3 || b.division == 1 {
            return (2, player.salary.wrapping_mul(2));
        }
    } else if (seller >= 3 && b.reputation >= 3) || (seller == 3 && (b.reputation == 2 || b.reputation == 1)) || seller <= 2 {
        return (0, 0);
    }
    (1, 0)
}

/// `ah.gg()`: the confederation of the club's country.
fn confederation(world: &World, club: ClubId) -> i32 {
    brasfoot_model::countries::confederation(world.club(club).country)
}

/// `l.a(F, ah, int)`: the seller's price from its depth at the player's position, then the
/// player's will. Prices are in `int`, as in the original.
pub fn offer(world: &World, p: PlayerId, buyer: ClubId, amount: i32) -> OfferAnswer {
    let player = world.player(p);
    let seller = player.club.expect("an offer is for a player with a club");
    let counts = squad_counts(world, seller);
    const ENOUGH: [i32; 5] = [3, 4, 4, 5, 4];
    const SCARCE: [f64; 6] = [1.5, 1.5, 1.5, 1.5, 2.0, 1.0];
    const SHORT: [f64; 6] = [1.0, 1.0, 1.0, 1.0, 1.5, 2.0];
    const PREMIUM: [f64; 6] = [0.5, 0.2, 0.2, 0.2, 0.5, 1.0];
    const DISCOUNT: [i32; 6] = [15, 20, 20, 10, 10, 2];
    let position = player.position as usize;
    let value = player.market_value;
    let round = |share: f64| (value as f64 * share).round() as i64 as i32;
    let price = if counts[position] >= ENOUGH[position] {
        if player.strength >= 30 && player.age <= 35 {
            value.wrapping_add(round(PREMIUM[position]))
        } else {
            // Math.round((float) (fk() * var8[pos] / 100)), the product in int
            value.wrapping_sub((value.wrapping_mul(DISCOUNT[position]) / 100) as f32 as i32)
        }
    } else if counts[position] == 1 {
        value.wrapping_add(round(SCARCE[position]))
    } else {
        value.wrapping_add(round(SHORT[position]))
    };
    if amount >= price {
        return match willingness(world, p, buyer) {
            (1, _) => OfferAnswer::Unwilling,
            (2, salary) => OfferAnswer::WantsSalary(salary),
            _ => OfferAnswer::Accepted,
        };
    }
    if world.club(buyer).cash >= price as i64 {
        match willingness(world, p, buyer) {
            (0, _) => return OfferAnswer::Counter { price, salary: None },
            (2, salary) => return OfferAnswer::Counter { price, salary: Some(salary) },
            _ => {}
        }
    }
    OfferAnswer::Refused
}

/// `ah.J(false)`: the squad by position, loans left out (the sixth count is the holding
/// midfielders).
fn squad_counts(world: &World, club: ClubId) -> [i32; 6] {
    let mut counts = [0; 6];
    for &p in &world.club(club).squad {
        let x = world.player(p);
        if !x.on_loan {
            counts[x.position as usize] += 1;
            if x.position == 3 && x.style == 0 {
                counts[5] += 1;
            }
        }
    }
    counts
}

impl Sim<'_> {
    /// `l.b(F, ah)`: the purchase of a listed player at his price.
    pub fn buy_listed(&mut self, p: PlayerId, buyer: ClubId) -> ListedAnswer {
        let player = self.world.player(p);
        if !player.listed_for_sale {
            return ListedAnswer::NotForSale;
        }
        if player.club == Some(buyer) {
            return ListedAnswer::OwnPlayer;
        }
        let price = player.price;
        if self.world.club(buyer).cash < price as i64 {
            return ListedAnswer::NoMoney;
        }
        if self.world.club(buyer).squad.len() >= SQUAD_CAP {
            return ListedAnswer::SquadLimit;
        }
        if willingness(self.world, p, buyer).0 == 1 {
            return ListedAnswer::Unwilling;
        }
        self.transfer(p, buyer, price);
        ListedAnswer::Bought
    }

    /// `cz.oL()` once the deal is done: the player moves for `fee`, with the salary he asked if
    /// any (`F.ae(int)`), and the seller fills the positions it is now short of (`l.a(ah, F)`).
    pub fn complete_purchase(&mut self, p: PlayerId, buyer: ClubId, fee: i32, salary: Option<i32>) {
        if let Some(salary) = salary {
            self.world.player_mut(p).salary = salary;
        }
        let seller = self.world.player(p).club.expect("a purchase is from a club");
        self.transfer(p, buyer, fee);
        self.refill_after_sale(seller, p);
    }

    /// `cz.oL()` when the player's club is another human's and he accepts: the player moves.
    pub fn complete_sale_between_humans(&mut self, p: PlayerId, buyer: ClubId, fee: i32) {
        self.transfer(p, buyer, fee);
    }

    /// `l.a(ah, F)`: for each position the seller is short of ({2, 3, 3, 5, 3}), it signs a player
    /// of the sold one's position and strength (the original passes his, whatever the position).
    fn refill_after_sale(&mut self, club: ClubId, sold: PlayerId) {
        const MINIMUM: [i32; 5] = [2, 3, 3, 5, 3];
        let (position, strength) = {
            let x = self.world.player(sold);
            (x.position, x.strength)
        };
        let counts = squad_counts(self.world, club);
        for (i, &minimum) in MINIMUM.iter().enumerate() {
            if counts[i] < minimum {
                self.sign_for_position(club, position, strength);
            }
        }
    }
}
