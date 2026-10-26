// Debug: the competitions' legacy ids of a save.
fn main() {
    let w = brasfoot_persistence::import::import_s22(std::fs::read(std::env::args().nth(1).unwrap()).unwrap()).unwrap();
    for (i, c) in w.competitions.iter().enumerate() {
        println!("{i} {:?} {} id={}", c.class, c.name, c.legacy_id);
    }
}
