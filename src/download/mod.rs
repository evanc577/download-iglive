mod backwards;
mod forwards;
mod initialization;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::future;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{IntoUrl, Url};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use self::forwards::download_forwards;
use self::initialization::download_reps_init;
use crate::download::backwards::download_reps_backwards;
use crate::error::IgtvError;
use crate::mpd::{Mpd, Representation};
use crate::state::State;

/// Download an IGTV stream.
/// Returns the download output path.
///
/// # Arguments
///
/// * `mpd_url` - Full URL of live stream's .mpd manifest.
/// * `dir` - Directory to place downloaded segments. If `None` is given, auto generate directory
/// based on live stream ID.
pub async fn download(mpd_url: impl IntoUrl, dir: Option<impl AsRef<Path>>) -> Result<PathBuf> {
    // Download manifest
    let url_base = mpd_url.into_url()?;
    let manifest = Mpd::from_url(url_base.clone()).await?;
    let (video_rep, audio_rep) = manifest.best_media();

    // Create directory
    let base_dir_name: PathBuf = if let Some(d) = dir {
        d.as_ref().into()
    } else {
        manifest.id.clone().into()
    };
    let dir_name = base_dir_name.join("segments");
    fs::create_dir_all(&dir_name).await?;

    // Create state
    let state = Arc::new(Mutex::new(State::new()));

    // Download initialization
    println!("Downloading initialization");
    download_reps_init(state.clone(), &url_base, [video_rep, audio_rep], &dir_name).await?;

    // Download current rep
    println!("Downloading current segments");
    download_reps(state.clone(), &url_base, [video_rep, audio_rep], &dir_name).await?;

    println!("Downloading past and live segments");

    // Progressbar
    let m = MultiProgress::new();
    let spinner_style =
        ProgressStyle::with_template("{prefix:.bold.fg.green} {spinner} {wide_msg}")?;
    let pb_video = m.add(ProgressBar::new_spinner());
    pb_video.set_style(spinner_style.clone());
    pb_video.set_prefix("Past video:");
    let pb_audio = m.add(ProgressBar::new_spinner());
    pb_audio.set_style(spinner_style.clone());
    pb_audio.set_prefix("Past audio:");
    let pb_forwards = m.add(ProgressBar::new_spinner());
    pb_forwards.set_style(spinner_style.clone());
    pb_forwards.set_prefix("      Live:");

    // Download backwards
    let (result_backwards, result_forwards) = tokio::join!(
        download_reps_backwards(
            state.clone(),
            &url_base,
            [(video_rep, &pb_video), (audio_rep, &pb_audio)],
            manifest.start_frame,
            &dir_name,
        ),
        download_forwards(state.clone(), &url_base, &dir_name, &pb_forwards),
    );
    result_backwards?;
    result_forwards?;

    Ok(base_dir_name)
}

async fn download_reps(
    state: Arc<Mutex<State>>,
    url_base: &Url,
    reps: impl IntoIterator<Item = &Representation>,
    dir: impl AsRef<Path> + Send,
) -> Result<()> {
    let futures: Vec<_> = reps
        .into_iter()
        .map(|rep| download_rep(state.clone(), rep, url_base, dir.as_ref()))
        .collect();
    future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<_>>()?;
    Ok(())
}

async fn download_rep(
    state: Arc<Mutex<State>>,
    rep: &Representation,
    url_base: &Url,
    dir: impl AsRef<Path>,
) -> Result<()> {
    let media_type = rep.media_type();
    for segment in &rep.segment_template.segment_timeline.segments {
        let t = segment.t;

        // Check if already downloaded
        if state.lock().await.downloaded_segs[&media_type]
            .get(&t)
            .is_some()
        {
            continue;
        }

        // Try to download segment
        let url = rep.download_url(url_base, t)?;
        let filename = dir.as_ref().join(
            url.path_segments()
                .ok_or(IgtvError::InvalidUrl)?
                .rev()
                .next()
                .ok_or(IgtvError::InvalidUrl)?,
        );
        download_file(&url, filename).await?;

        // Update state
        state
            .lock()
            .await
            .downloaded_segs
            .get_mut(&media_type)
            .unwrap()
            .insert(t);
    }
    Ok(())
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
