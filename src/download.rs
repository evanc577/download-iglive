mod backwards;
mod forwards;
mod initialization;

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Url;
use tokio::io::AsyncWriteExt;
use tokio::fs;
use tokio::sync::Mutex;

use self::forwards::download_reps_forwards;
use self::initialization::download_reps_init;
use crate::download::backwards::download_reps_backwards;
use crate::mpd::Mpd;
use crate::state::State;

pub struct Downloader {
    t: usize,
}

impl Downloader {
    pub async fn download(mpd_url: &str) -> Result<()> {
        // Download manifest
        let manifest = Mpd::from_url(mpd_url).await?;
        dbg!(&manifest);
        let (video_rep, audio_rep) = manifest.best_media();

        // Create directory
        let dir_name = &manifest.id;
        fs::create_dir_all(dir_name).await?;

        // Create state
        let state = Arc::new(Mutex::new(State::new()));

        // Download initialization
        let url_base = Url::parse(mpd_url)?;
        println!("Downloading initialization");
        download_reps_init(state.clone(), &url_base, [video_rep, audio_rep], dir_name).await?;

        println!("Downloading previous segments");

        // Progressbar
        let m = MultiProgress::new();
        let spinner_style =
            ProgressStyle::with_template("{prefix:.bold.fg.green} {spinner} {wide_msg}")?;
        let pb_video = m.add(ProgressBar::new_spinner());
        pb_video.set_style(spinner_style.clone());
        pb_video.set_prefix("Video:");
        let pb_audio = m.add(ProgressBar::new_spinner());
        pb_audio.set_style(spinner_style.clone());
        pb_audio.set_prefix("Audio:");

        // Download backwards
        download_reps_backwards(
            state.clone(),
            &url_base,
            [(video_rep, &pb_video), (audio_rep, &pb_audio)],
            manifest.start_frame,
            dir_name,
        )
        .await?;

        Ok(())
    }
}

async fn download_file(url: &Url, path: impl AsRef<Path>) -> Result<()> {
    let resp = reqwest::get(url.as_str()).await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Failed to download {}", url.as_str()));
    }

    let mut buffer = fs::File::create(path).await?;
    buffer.write_all(&resp.bytes().await?).await?;
    Ok(())
}
