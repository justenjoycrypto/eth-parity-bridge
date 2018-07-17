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
use std::path::PathBuf;
use std::{env, fs};
use tokio_core::reactor::Core;
use web3::transports::http::Http;

use bridge::config::Config;
use bridge::database::State;
use bridge::deploy::{DeployForeign, DeployHome};
use bridge::error::{self, ResultExt};

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
        .join("\n\nCaused by:\n	");
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
    parity-bridge-deploy --config <config> --database <database>
    parity-bridge-deploy -h | --help

Options:
    -h, --help           Display help message and exit.
"#,
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    );

    info!(target: "parity-bridge-deploy", "Parsing cli arguments");
    let args: Args = Docopt::new(usage)
        .and_then(|d| d.argv(command).deserialize())
        .map_err(|e| e.to_string())?;

    info!(target: "parity-bridge-deploy", "Loading config");
    let config = Config::load(args.arg_config)?;

    info!(target: "parity-bridge-deploy", "Starting event loop");
    let mut event_loop = Core::new().unwrap();

    info!(
        "Establishing HTTP connection to home {:?}",
        config.home.http
    );
    let home_transport =
        Http::with_event_loop(
            &config.home.http,
            &event_loop.handle(),
            MAX_PARALLEL_REQUESTS,
        ).chain_err(|| format!("Cannot connect to home at {}", config.home.http))?;

    info!(
        "Establishing HTTP connection to foreign {:?}",
        config.foreign.http
    );
    let foreign_transport =
        Http::with_event_loop(
            &config.foreign.http,
            &event_loop.handle(),
            MAX_PARALLEL_REQUESTS,
        ).chain_err(|| format!("Cannot connect to foreign at {}", config.foreign.http))?;

    info!(target: "parity-bridge-deploy", "Deploying HomeBridge contract");
    let home_deployed = event_loop.run(DeployHome::new(config.clone(), home_transport))?;
    info!(target: "parity-bridge-deploy", "Successfully deployed HomeBridge contract");

    home_deployed.dump_info(format!(
        "deployment-home-{}",
        home_deployed.contract_address
    ))?;

    info!(target: "parity-bridge-deploy", "Deploying ForeignBridge contract");
    let foreign_deployed = event_loop.run(DeployForeign::new(config.clone(), foreign_transport))?;
    info!(target: "parity-bridge-deploy", "Successfully deployed ForeignBridge contract");

    foreign_deployed.dump_info(format!(
        "deployment-foreign-{}",
        foreign_deployed.contract_address
    ))?;

    let state = State::from_transaction_receipts(&home_deployed.receipt, &foreign_deployed.receipt);
    info!(target: "parity-bridge-deploy", "\n\n{}\n", state);
    state.write(fs::File::create(args.arg_database)?)?;

    Ok("Done".into())
}
