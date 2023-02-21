use std::time::Instant;

use cadabra::*;

fn main() {
    let pos = Position::start_pos();

    let depth = 7;

    let before = Instant::now();

    println!(" Found: {} moves at depth {depth} in {}ms", pos.perft::<true>(depth), before.elapsed().as_millis());
}