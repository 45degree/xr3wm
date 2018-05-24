#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate fern;
#[macro_use]
extern crate log;

extern crate xcb;

mod errors;
mod window_system;

use errors::*;
use window_system::{WindowSystem, XWindowSystem};

fn init_logger() -> Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                    "[{}] {}",
                    record.level(),
                    message))
        })
        .chain(std::io::stdout())
        .apply()
        .map_err(|e| e.into())
}

fn run() -> Result<()> {
    init_logger()?;

    info!("initializing logger");

    let backend = XWindowSystem::initialize()
        .chain_err(|| "unable to initialize Window System")?;

    backend.run()
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);
        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        std::process::exit(1);
    }
}
