use std::path::PathBuf;

use clap::Parser;

#[derive(Debug)]
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long = "db", default_value = "./db.sqlite")]
    pub db_path: PathBuf,
    #[arg(long = "mount", default_value = "./xkcd/")]
    pub mount_point: PathBuf,
    #[arg(long = "start", default_value_t = u32::MAX)]
    pub start: u32,
    #[arg(long = "end", default_value_t = u32::MAX)]
    pub end: u32,
}
