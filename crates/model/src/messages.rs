//! Messages whose texts the original composes in Portuguese from game data (a stage's name, an
//! amount of money written out). The port keeps the data in the message ([`Message::stage`], [`Message::competition`],
//! [`Message::money`], [`Message::confederation`]) and leaves those texts empty: the screens compose them from the catalog.

use crate::saved::Message;
use crate::world::World;

/// `(sender, message_id)` of the messages whose `text1` and `text2` are composed.
pub const COMPOSED_TEXTS: &[(i32, i32)] = &[
    // f.G.V(ah): "<stage>" and the prize of a knockout round won
    (26, 79),
    // ah.c(at, int): "<competition>" and the prize of a league place
    (26, 80),
    // al.q(at): the continental cup of a title, " da Libertadores"
    (25, 76),
];

fn composed(m: &Message) -> bool {
    COMPOSED_TEXTS.contains(&(m.sender, m.message_id))
}

/// For comparing with the original: blanks the composed texts of every coach's messages.
pub fn blank_composed_texts(world: &mut World) {
    for coach in &mut world.coaches {
        for m in coach.messages.iter_mut().filter(|m| composed(m)) {
            m.text1.clear();
            m.text2.clear();
        }
    }
}

/// For comparing with the original: the date text it writes in each message (`best.a.f()`,
/// "dd/MM/yyyy" of the day), from the day the port keeps.
pub fn fill_date_texts(world: &mut World, clock: &crate::clock::Clock) {
    for coach in &mut world.coaches {
        for m in &mut coach.messages {
            if let Some(day) = m.day
                && m.date_text.is_empty()
            {
                let f = clock.fields(day);
                m.date_text = format!("{:02}/{:02}/{:04}", f.day, f.month + 1, f.year);
            }
        }
    }
}
