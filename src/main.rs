use std::path::PathBuf;

use clap::{Parser, Subcommand};
use merge::merge;

use crate::download::Downloader;

mod download;
mod error;
mod mpd;
mod state;
mod merge;

/// Download Instagram live streams (IGTV), including past segments
#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Download(Download),
    Merge(Merge),
}

/// Download a live stream
#[derive(Parser, Debug,)]
struct Download {
        /// URL of .mpd file
        mpd_url: String,

        /// Output directory
        #[clap(short, long)]
        output: Option<PathBuf>,

        /// Don't merge into one video file after download
        #[clap(short, long)]
        no_merge: bool,
}

/// Merge an already downloaded live stream into one file
#[derive(Parser, Debug,)]
struct Merge {
    /// Directory to merge
    directory: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Download(d) => {
            let output_dir = Downloader::download(&d.mpd_url, d.output).await.unwrap();
            if !d.no_merge {
                merge(output_dir).unwrap();
            }
        }
        Command::Merge(m) => merge(m.directory).unwrap(),
    }
}
