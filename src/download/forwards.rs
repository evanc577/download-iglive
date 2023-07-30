use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use futures::future;
use indicatif::ProgressBar;
use reqwest::{Client, Url};
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

use crate::download::download_rep;
use crate::mpd::{MediaType, Mpd, Representation};
use crate::state::State;

pub async fn download_forwards(
    state: Arc<Mutex<State>>,
    client: &Client,
    url_base: &Url,
    dir: impl AsRef<Path> + Send,
    pb: Option<ProgressBar>,
) -> Result<()> {
    // Set up 2 second interval
    let mut interval = time::interval(Duration::from_millis(1000));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let ret = loop {
        // Wait for interval
        interval.tick().await;

        // Download manifest
        let manifest = Mpd::download_from_url(client, url_base).await?;
        let (video_rep, audio_rep) = manifest.best_media();

        // Find last segments downloaded
        let (latest_video_t, latest_audio_t) = {
            let segs = &state.lock().await.downloaded_segs;
            let latest_video_t = *segs[&MediaType::Video].iter().max().unwrap();
            let latest_audio_t = *segs[&MediaType::Audio].iter().max().unwrap();
            (latest_video_t, latest_audio_t)
        };

        // Download reps
        let futures: Vec<_> = [video_rep, audio_rep]
            .into_iter()
            .map(|rep| download_rep(state.clone(), client, rep, url_base, dir.as_ref()))
            .collect();
        future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<_>>()?;

        check_overlap(video_rep, latest_video_t, &pb);
        check_overlap(audio_rep, latest_audio_t, &pb);

        // Update progress bar
        if let Some(pb) = pb.as_ref() {
            pb.set_message(format!(
                "Downloaded video segment {}, audio segment {}",
                latest_video_t, latest_audio_t
            ));
            pb.tick();
        }

        // Finish if stream ended
        if manifest.finished {
            break Ok(());
        }
    };

    if let Some(pb) = pb {
        pb.finish_with_message("Finished");
    }

    ret
}

fn check_overlap(rep: &Representation, latest_t: usize, pb: &Option<ProgressBar>) {
    if !rep
        .segment_template
        .segment_timeline
        .segments
        .iter()
        .any(|s| s.t == latest_t)
    {
        let msg = format!("Possible missed live segment t={latest_t}");
        if let Some(pb) = pb.as_ref() {
            pb.println(msg);
        } else {
            eprintln!("{msg}");
        }
    }
}
