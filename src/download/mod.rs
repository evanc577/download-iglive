mod backwards;
mod forwards;
mod initialization;

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use bitflags::bitflags;
use futures::{future, Future};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{Client, IntoUrl, StatusCode, Url};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use self::backwards::download_reps_backwards;
use self::forwards::download_forwards;
use self::initialization::download_reps_init;
use crate::error::IgLiveError;
use crate::mpd::{MediaType, Mpd, Representation};
use crate::pts::get_pts;
use crate::state::State;

/// Options for download
#[derive(Clone, Debug)]
pub struct DownloadConfig {
    /// Directory to place downloaded segments.
    /// If `None`, auto generate directory based on live stream ID.
    pub dir: Option<PathBuf>,

    /// Choose whether to download live segments or past segments.
    pub segments: DownloadSegments,
}

bitflags! {
    /// Types of segments to download
    pub struct DownloadSegments: u32 {
        /// Download live segments.
        const LIVE = 0b00000001;

        /// Download past segments.
        const PAST = 0b00000010;
    }
}

/// Download an IG live stream.
/// Returns the download output path.
///
/// # Arguments
///
/// * `mpd_url` - Full URL of live stream's .mpd manifest.
pub async fn download(mpd_url: impl IntoUrl, config: DownloadConfig) -> Result<PathBuf> {
    // Reqwest client
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

    // Download manifest
    let url_base = mpd_url.into_url()?;
    let manifest = Mpd::download_from_url(&client, url_base.clone()).await?;
    let (video_rep, audio_rep) = manifest.best_media();

    // Create directory
    let base_dir_name: PathBuf = if let Some(d) = config.dir {
        d
    } else {
        manifest.id.clone().into()
    };
    let dir_name = base_dir_name.join("segments");
    fs::create_dir_all(&dir_name).await?;

    // Create state
    let state = Arc::new(Mutex::new(State::new()));

    // Progress bar
    let m = MultiProgress::new();
    let spinner_style =
        ProgressStyle::with_template("{prefix:.bold.fg.green} {spinner} {wide_msg}")?;

    // Download initialization
    let pb_init = m.add(ProgressBar::new_spinner());
    pb_init.enable_steady_tick(Duration::from_millis(500));
    pb_init.set_style(spinner_style.clone());
    pb_init.set_prefix("      Init");
    download_reps_init(
        state.clone(),
        &client,
        &url_base,
        [video_rep, audio_rep],
        Some(pb_init),
    )
    .await?;

    // Download current rep
    let pb_current = m.add(ProgressBar::new_spinner());
    pb_current.enable_steady_tick(Duration::from_millis(500));
    pb_current.set_style(spinner_style.clone());
    pb_current.set_prefix("   Current");
    download_reps(
        state.clone(),
        &client,
        &url_base,
        [video_rep, audio_rep],
        &dir_name,
        Some(pb_current),
    )
    .await?;

    let pb_forwards;
    let pb_video;
    let pb_audio;

    // Download past and live segments
    let mut futures: Vec<Pin<Box<dyn Future<Output = Result<()>>>>> = vec![];
    if config.segments.contains(DownloadSegments::LIVE) {
        // Download live segments
        let pb_forwards_tmp = m.add(ProgressBar::new_spinner());
        pb_forwards_tmp.set_style(spinner_style.clone());
        pb_forwards_tmp.set_prefix("      Live");
        pb_forwards = Some(pb_forwards_tmp);

        futures.push(Box::pin(download_forwards(
            state.clone(),
            &client,
            &url_base,
            &dir_name,
            pb_forwards,
        )));
    }
    if config.segments.contains(DownloadSegments::PAST) {
        // Download past segments
        let pb_video_tmp = m.add(ProgressBar::new_spinner());
        pb_video_tmp.set_style(spinner_style.clone());
        pb_video_tmp.set_prefix("Past video");
        pb_video = Some(pb_video_tmp);
        let pb_audio_tmp = m.add(ProgressBar::new_spinner());
        pb_audio_tmp.set_style(spinner_style.clone());
        pb_audio_tmp.set_prefix("Past audio");
        pb_audio = Some(pb_audio_tmp);

        futures.push(Box::pin(download_reps_backwards(
            state.clone(),
            &client,
            &url_base,
            [(video_rep, pb_video), (audio_rep, pb_audio)],
            manifest.start_frame,
            &dir_name,
        )));
    }
    future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<_>>()?;

    Ok(base_dir_name)
}

async fn download_reps(
    state: Arc<Mutex<State>>,
    client: &Client,
    url_base: &Url,
    reps: impl IntoIterator<Item = &Representation>,
    dir: impl AsRef<Path> + Send,
    pb: Option<ProgressBar>,
) -> Result<()> {
    if let Some(pb) = pb.as_ref() {
        pb.set_message("Downloading");
    }

    let futures: Vec<_> = reps
        .into_iter()
        .map(|rep| download_rep(state.clone(), client, rep, url_base, dir.as_ref()))
        .collect();
    future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<_>>()?;

    if let Some(pb) = pb.as_ref() {
        pb.finish_with_message("Finished");
    }

    Ok(())
}

async fn download_rep(
    state: Arc<Mutex<State>>,
    client: &Client,
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
                .ok_or(IgLiveError::InvalidUrl)?
                .rev()
                .next()
                .ok_or(IgLiveError::InvalidUrl)?,
        );
        download_file(
            state.clone(),
            client,
            rep.media_type(),
            false,
            &url,
            filename,
        )
        .await?;

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

async fn download_file(
    state: Arc<Mutex<State>>,
    client: &Client,
    media_type: MediaType,
    check_pts: bool,
    url: &Url,
    path: impl AsRef<Path>,
) -> Result<()> {
    let resp = client.get(url.as_str()).send().await?;
    if resp.status() == StatusCode::NOT_FOUND {
        return Err(IgLiveError::StatusNotFound.into());
    }
    if !resp.status().is_success() {
        return Err(anyhow!("Failed to download {}", url.as_str()));
    }

    let mut buffer = Vec::new();
    buffer
        .write_all(state.lock().await.downloaded_init.get(&media_type).unwrap())
        .await?;
    buffer.write_all(&resp.bytes().await?).await?;

    // Write to file
    let mut file_buffer = fs::File::create(path).await?;
    file_buffer.write_all(&buffer).await?;

    // Check pts
    let pts = get_pts(buffer).await.unwrap();
    if check_pts {
        let target_pts = *state.lock().await.back_pts.get(&media_type).unwrap();
        if target_pts != pts.1 {
            return Err(IgLiveError::PtsTooEarly.into());
        }
    }

    // Update pts
    state
        .lock()
        .await
        .back_pts
        .entry(media_type)
        .and_modify(|p| *p = std::cmp::min(*p, pts.0))
        .or_insert(pts.0);

    Ok(())
}
