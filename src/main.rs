#![feature(let_chains)]

use std::io::Write;

use clap::Parser;
use log::LevelFilter;

use crate::{
    cli::Cli,
    storage::{BlockingXkcdStorage, XkcdStorage, XkcdStorageConfig},
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

fn main() -> Result<(), ()> {
    init_logger();
    let cli = Cli::parse();

    let storage: XkcdStorage = XkcdStorageConfig { db_path: cli.db_path }.into();
    let blocking_storage: BlockingXkcdStorage = storage.into();
    blocking_storage.ensure_range(cli.start, cli.end)?;
    fs::fuse(cli.mount_point.as_path(), blocking_storage);
    Ok(())
}
