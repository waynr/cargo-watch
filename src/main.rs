//! Watch files in a Cargo project and compile it when they change

extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate notify;
extern crate regex;
extern crate rustc_serialize;


use docopt::Docopt;
use notify::{Error, RecommendedWatcher, Watcher};
use std::sync::mpsc::channel;

mod cargo;
mod schedule;

static USAGE: &'static str = r#"
Usage: cargo-watch [watch] [options]
       cargo watch [options]
       cargo-watch [watch] [<args>...]
       cargo watch [<args>...]

Options:
  -h, --help      Display this message

`cargo watch` can take one or more arguments to pass to cargo. For example,
`cargo watch "test ex_ --release"` will run `cargo test ex_ --release`

If no arguments are provided, then cargo will run `build` and `test`
"#;

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_args: Vec<String>,
}

fn main() {
    // Initialize logger
    env_logger::init().unwrap();

    // Parse CLI parameters
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());
    let commands = args.arg_args;

    // Check if we are (somewhere) in a cargo project directory
    let cargo_dir = match cargo::root() {
        Some(path) => path,
        None => {
            error!("Not a Cargo project, aborting.");
            std::process::exit(64);
        },
    };

    // Creates `Watcher` instance and a channel to communicate with it
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = match Watcher::new(tx) {
        Ok(i) => i,
        Err(e) => {
            error!("Failed to init notify ({:?})", e);
            std::process::exit(1);
        },
    };

    // Configure watcher: we want to watch these subfolders
    {
        // FIXME: using a closure here to be able to use `try!` here. Using
        // `and_then` is even uglier IMO. Waiting for the "try-catch" RFC.
        let mut add_dirs = || -> Result<(), notify::Error> {
            try!(watcher.watch(&cargo_dir.join("src")));
            try!(watcher.watch(&cargo_dir.join("tests")));
            try!(watcher.watch(&cargo_dir.join("benches")));
            Ok(())
        };
        if let Err(e) = add_dirs() {
            error!("Failed to watch some folders with `notify`: {:?}", e);
            std::process::exit(2);
        }
    }

    // Tell the user that we are ready
    println!("Waiting for changes... Hit Ctrl-C to stop.");

    // Handle incoming events from the watcher
    schedule::handle(rx, commands);
}
