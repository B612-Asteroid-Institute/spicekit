//! Read positions and velocities from an SPK kernel.
//!
//! Run with:
//!   cargo run --example read_spk -- path/to/de440.bsp
//!
//! Prints the state (km, km/s) of Earth (NAIF ID 399) relative to the
//! Solar System Barycenter (NAIF ID 0) at J2000 epoch (et = 0.0).

use std::env;

use spicekit::SpkFile;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <path-to-spk>", args[0]);
        std::process::exit(1);
    }
    let spk = SpkFile::open(&args[1]).expect("open SPK");
    let state = spk.state(399, 0, 0.0).expect("Earth state at J2000");
    println!(
        "Earth (399) rel SSB (0) at ET=0.0:\n  \
         position = ({:+.3}, {:+.3}, {:+.3}) km\n  \
         velocity = ({:+.6}, {:+.6}, {:+.6}) km/s",
        state[0], state[1], state[2], state[3], state[4], state[5],
    );
}
