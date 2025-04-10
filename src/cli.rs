use std::path::PathBuf;

use clap::Parser;

#[derive(Debug)]
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(
        long = "db",
        default_value = "./db.sqlite",
        help = "Path to the SQLite database file"
    )]
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub db_path: PathBuf,
    #[arg(long = "mount", default_value = "./xkcd/", help = "Mount point for the XkcdFS")]
    #[arg(value_hint = clap::ValueHint::DirPath)]
    pub mount_point: PathBuf,
    #[arg(long = "start", default_value_t = u32::MAX, help = "Start of the range to fetch")]
    pub start: u32,
    #[arg(long = "end", default_value_t = u32::MAX, help = "End of the range to fetch")]
    pub end: u32,
}
