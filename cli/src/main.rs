extern crate bridge;
extern crate docopt;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate web3;

use docopt::Docopt;
use futures::{future, Stream};
use std::env;
use std::path::PathBuf;
use tokio_core::reactor::Core;
use web3::transports::http::Http;

use bridge::bridge::Bridge;
use bridge::config::Config;
use bridge::database::{Database, TomlFileDatabase};
use bridge::error::{self, ResultExt};
use bridge::helpers::StreamExt;
use bridge::main_contract::MainContract;
use bridge::side_contract::SideContract;

const MAX_PARALLEL_REQUESTS: usize = 10;

#[derive(Debug, Deserialize)]
pub struct Args {
    arg_config: PathBuf,
    arg_database: PathBuf,
}

fn main() {
    let _ = env_logger::init();
    let result = execute(env::args());

    match result {
        Ok(s) => println!("{}", s),
        Err(err) => print_err(err),
    }
}

fn print_err(err: error::Error) {
    let message = err.iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n\nCaused by:\n  ");
    println!("{}", message);
}

fn execute<S, I>(command: I) -> Result<String, error::Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let usage = format!(
        r#"
Parity-bridge
    Copyright 2017 Parity Technologies (UK) Limited
    Version: {}
    Commit: {}

Usage:
    parity-bridge --config <config> --database <database>
    parity-bridge -h | --help

Options:
    -h, --help           Display help message and exit.
"#,
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );

    info!("Parsing cli arguments");
    let args: Args = Docopt::new(usage)
        .and_then(|d| d.argv(command).deserialize())
        .map_err(|e| e.to_string())?;

    info!("Loading config from {:?}", args.arg_config);
    let config = Config::load(args.arg_config)?;

    info!("Starting event loop");
    let mut event_loop = Core::new().unwrap();

    info!(
        "Establishing HTTP connection to main {:?}",
        config.home.http
    );
    let main_transport =
        Http::with_event_loop(
            &config.home.http,
            &event_loop.handle(),
            MAX_PARALLEL_REQUESTS,
        ).chain_err(|| format!("Cannot connect to main at {}", config.home.http))?;

    info!(
        "Establishing HTTP connection to side {:?}",
        config.foreign.http
    );
    let side_transport =
        Http::with_event_loop(
            &config.foreign.http,
            &event_loop.handle(),
            MAX_PARALLEL_REQUESTS,
        ).chain_err(|| format!("Cannot connect to side at {}", config.foreign.http))?;

    info!("Loading database from {:?}", args.arg_database);
    let mut database = TomlFileDatabase::from_path(&args.arg_database)?;

    info!("Reading initial state from database");
    let initial_state = database.read();

    let main_contract = MainContract::new(main_transport.clone(), &config, &initial_state);

    let side_contract = SideContract::new(main_transport.clone(), &config, &initial_state);

    let bridge_stream = Bridge::new(initial_state, main_contract, side_contract);
    info!("Listening to events");
    let persisted_bridge_stream = bridge_stream.and_then(|state| {
        database.write(&state)?;
        // info!("state change: {}", state);
        Ok(())
    });

    event_loop.run(persisted_bridge_stream.last())?;

    Ok("Done".into())
}
