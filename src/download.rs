mod backwards;
mod forwards;
mod initialization;

use std::path::Path;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Url;
use tokio::io::AsyncWriteExt;
use tokio::{fs, join};

use self::backwards::download_backwards;
use self::forwards::download_forwards;
use self::initialization::download_initialization;
use crate::mpd::Mpd;

pub struct Downloader {
    t: usize,
}

impl Downloader {
    pub async fn download(mpd_url: &str) {
        // Download manifest
        let manifest = Mpd::from_url(mpd_url).await;
        let (video_rep, audio_rep) = &manifest.best_media();

        // Create directory
        let dir_name = &manifest.id;
        fs::create_dir_all(dir_name).await.unwrap();

        // Download initialization
        let url_base = Url::parse(mpd_url).unwrap();
        println!("Downloading initialization");
        let ((), ()) = join!(
            download_initialization(&url_base, video_rep, dir_name),
            download_initialization(&url_base, audio_rep, dir_name),
        );

        println!("Downloading previous segments");

        // Progressbar
        let m = MultiProgress::new();
        let spinner_style =
            ProgressStyle::with_template("{prefix:.bold.fg.green} {spinner} {wide_msg}").unwrap();
        let pb_video = m.add(ProgressBar::new_spinner());
        pb_video.set_style(spinner_style.clone());
        pb_video.set_prefix("Video:");
        let pb_audio = m.add(ProgressBar::new_spinner());
        pb_audio.set_style(spinner_style.clone());
        pb_audio.set_prefix("Audio:");

        // Download backwards
        let ((), ()) = join!(
            download_backwards(&url_base, video_rep, dir_name, &pb_video),
            download_backwards(&url_base, audio_rep, dir_name, &pb_audio)
        );
    }
}

async fn download_file(url: &Url, path: impl AsRef<Path>) -> Result<(), ()> {
    let resp = reqwest::get(url.as_str()).await.unwrap();
    if !resp.status().is_success() {
        return Err(());
    }

    let mut buffer = fs::File::create(path).await.unwrap();
    buffer
        .write_all(&resp.bytes().await.unwrap())
        .await
        .unwrap();
    Ok(())
}
