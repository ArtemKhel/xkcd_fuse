#![feature(let_chains)]

use std::io::Write;

use clap::Parser;
use log::{LevelFilter, info};

use crate::{
    cli::Cli,
    storage::{XkcdStorage, XkcdStorageConfig},
};

mod api;
mod cli;
mod db;
mod fs;
mod storage;
mod xkcd;

fn init_logger() {
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "[{}]: {}", record.target(), record.args()))
        .filter_level(LevelFilter::Warn)
        .parse_default_env()
        .try_init()
        .unwrap();
}

// TODO:
//  - async
//  - better error handling
// #[tokio::main]
fn main() -> Result<(), ()> {
    init_logger();
    let cli = Cli::parse();
    info!("{cli:?}");

    let storage: XkcdStorage = XkcdStorageConfig { db_path: cli.db_path }.into();
    storage.ensure_range(cli.start, cli.end)?;
    fs::fuse(cli.mount_point.as_path(), storage);
    Ok(())
}
