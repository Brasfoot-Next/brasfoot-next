//! Plays days of a career with the human doing the least (a 4-4-2 or the first formation that
//! fills eleven, nothing during the matches) and saves it elsewhere, to look at the screens in
//! the middle of a season. Usage: advance_career <in.bfn> <out.bfn> <days>

use brasfoot_next::career::Career;
use brasfoot_next::game_files::GameFiles;
use brasfoot_next::view;
use brasfoot_simulation::human::lineup::LineupScreen;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (input, output, days) = (&args[1], &args[2], args.get(3).and_then(|d| d.parse().ok()).unwrap_or(10));
    let files = GameFiles::locate();
    let mut career = Career::open(std::path::Path::new(input), &files).expect("open");
    for day in 0..days {
        let Some(club) = career.human_club() else {
            if career.advance().is_err() {
                break;
            }
            continue;
        };
        let Some(m) = view::next_match(&career.world, club) else { break };
        let mut screen = LineupScreen::open(&mut career.world, club, m);
        for formation in std::iter::once(4).chain((2..13).filter(|&f| f != 4)) {
            screen.pick_formation(&career.world, formation);
            if screen.starters() == 11 {
                break;
            }
        }
        if screen.confirm(&mut career.world).is_err() {
            eprintln!("day {day}: no line-up");
            break;
        }
        match career.start_live_round() {
            Ok(mut round) => career.finish_live_round(&mut round),
            Err(e) => {
                eprintln!("day {day}: {e:?}");
                break;
            }
        }
        let _ = career.mod_tick();
        // the least a human does with the mod's proposals: the first master offer, no investor
        let c = &mut career;
        c.systems.with(&mut c.world, &mut c.runtime, |m| {
            if !m.world.mods.master_sponsor.offers.is_empty() {
                m.sign_master_sponsor(0);
            }
            if m.world.mods.investor.shown.is_some() {
                m.refuse_investor();
            }
        });
    }
    career.path = output.into();
    career.save(None).expect("save");
    println!("{} after {} days", output, days);
}
