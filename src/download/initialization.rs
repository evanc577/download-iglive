use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use futures::future;
use indicatif::ProgressBar;
use reqwest::Url;
use tokio::sync::Mutex;

use super::download_file;
use crate::error::IgLiveError;
use crate::mpd::Representation;
use crate::state::State;

pub async fn download_reps_init(
    state: Arc<Mutex<State>>,
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
        .map(|rep| download_init(state.clone(), url_base, rep, dir.as_ref()))
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

async fn download_init(
    state: Arc<Mutex<State>>,
    url_base: &Url,
    rep: &Representation,
    dir: impl AsRef<Path>,
) -> Result<()> {
    let media_type = rep.media_type();
    if state.lock().await.downloaded_init[&media_type] {
        return Ok(());
    }

    let url = url_base.join(&rep.segment_template.initialization_path)?;
    let filename = dir.as_ref().join(
        url.path_segments()
            .ok_or(IgLiveError::InvalidUrl)?
            .rev()
            .next()
            .ok_or(IgLiveError::InvalidUrl)?,
    );
    download_file(&url, filename).await?;

    state.lock().await.downloaded_init.insert(media_type, true);

    Ok(())
}
