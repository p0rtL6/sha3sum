use std::env;

use sha3sum::{Mode, Sponge};

fn main() {
    let mut mode = Mode::default();

    let mut args: Vec<String> = env::args().collect();
    args.remove(0);

    let args_clone: Vec<String> = args.clone();

    for i in 0..args_clone.len() {
        if args_clone[i] == "-m" {
            mode = Mode::try_from(
                args_clone
                    .get(i + 1)
                    .expect("Please provide a mode value (224, 256, 384, 512)"),
            )
            .expect("Invalid mode (224, 256, 384, 512)");

            args.remove(i);
            args.remove(i);
        }
    }

    for argument in args {
        let mut sponge: Sponge = Sponge::new(mode);

        sponge.absorb(&argument);
        println!("{}  {}", sponge.squeeze(), argument);
    }
}
