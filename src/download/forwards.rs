use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use futures::future;
use indicatif::ProgressBar;
use reqwest::Url;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

use crate::download::download_rep;
use crate::mpd::{MediaType, Mpd};
use crate::state::State;

pub async fn download_forwards(
    state: Arc<Mutex<State>>,
    url_base: &Url,
    dir: impl AsRef<Path> + Send,
    pb: &ProgressBar,
) -> Result<()> {
    // Set up 2 second interval
    let mut interval = time::interval(Duration::from_secs(2));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let ret = loop {
        // Wait for interval
        interval.tick().await;

        // Download manifest
        let manifest = Mpd::from_url(url_base).await?;
        let (video_rep, audio_rep) = manifest.best_media();

        // Download reps
        let futures: Vec<_> = [video_rep, audio_rep]
            .into_iter()
            .map(|rep| download_rep(state.clone(), rep, url_base, dir.as_ref()))
            .collect();
        future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<_>>()?;

        // Update progress bar
        let (latest_video_t, latest_audio_t) = {
            let segs = &state.lock().await.downloaded_segs;
            let latest_video_t = *segs[&MediaType::Video].iter().max().unwrap();
            let latest_audio_t = *segs[&MediaType::Audio].iter().max().unwrap();
            (latest_video_t, latest_audio_t)
        };
        pb.set_message(format!(
            "Downloaded video segment {}, audio segment {}",
            latest_video_t, latest_audio_t
        ));
        pb.tick();

        // Finish if stream ended
        if manifest.finished {
            break Ok(());
        }
    };

    pb.finish_with_message("Finished");

    ret
}
