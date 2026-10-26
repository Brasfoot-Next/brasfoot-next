//! Messages to a coach (`components.as`).

use brasfoot_model::ids::{CoachId, CompetitionId, StageId};
use brasfoot_model::saved::Message;
use brasfoot_model::world::World;

/// `new components.as(al, int, int, String, String)`: `sender` is the kind of message, `id` the
/// text it shows (`text1` and `text2` fill it in).
pub fn send(world: &mut World, coach: CoachId, sender: i32, id: i32, text1: String, text2: String) {
    let day = Some(world.today_millis());
    let message = Message { sender, message_id: id, text1, text2, day, ..Default::default() };
    world.coach_mut(coach).messages.push(message);
}

/// `new components.as(boolean, int, ah, F, al, int, int, String, String)`: a message that also
/// names a player and a tenth of his salary (`oq`), with its flag (`RZ`) and kind (`Sc`).
#[allow(clippy::too_many_arguments)]
pub fn send_about_player(
    world: &mut World,
    coach: CoachId,
    sender: i32,
    id: i32,
    text1: String,
    text2: String,
    player_name: String,
    salary: i32,
    flag: bool,
    kind: i32,
) {
    let day = Some(world.today_millis());
    let message = Message {
        sender,
        message_id: id,
        text1,
        text2,
        text3: player_name,
        amount: brasfoot_model::rng::java_round(salary as f64 * 0.1) as i32,
        flag_rz: flag,
        value_sc: kind,
        day,
        ..Default::default()
    };
    world.coach_mut(coach).messages.push(message);
}

/// What a message with composed texts names.
#[derive(Clone, Copy, Debug, Default)]
pub struct Composed {
    pub stage: Option<StageId>,
    pub competition: Option<CompetitionId>,
    pub money: Option<i64>,
}

/// `new components.as(al, int, int, String, String)` of a message whose texts the original
/// composes from a name and an amount (see [`brasfoot_model::messages`]): the port keeps
/// the stage and the amount instead.
pub fn send_composed(world: &mut World, coach: CoachId, sender: i32, id: i32, about: Composed) {
    let day = Some(world.today_millis());
    let message = Message { sender, message_id: id, day, stage: about.stage, competition: about.competition, money: about.money, ..Default::default() };
    world.coach_mut(coach).messages.push(message);
}

/// `new components.as(al, 25, 76, "", cup)` of `al.q(at)`: the board's congratulations on a
/// continental title, the cup of the club's confederation named in the text (" da
/// Libertadores"); the port keeps the confederation.
pub fn send_continental_title(world: &mut World, coach: CoachId, confederation: i32) {
    let day = Some(world.today_millis());
    let message = Message { sender: 25, message_id: 76, day, confederation: (0..6).contains(&confederation).then_some(confederation), ..Default::default() };
    world.coach_mut(coach).messages.push(message);
}
