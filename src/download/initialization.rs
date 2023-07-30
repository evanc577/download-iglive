use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::future;
use indicatif::ProgressBar;
use reqwest::{Url, Client, StatusCode};
use tokio::sync::Mutex;

use crate::error::IgLiveError;
use crate::mpd::Representation;
use crate::state::State;

pub async fn download_reps_init(
    state: Arc<Mutex<State>>,
    client: &Client,
    url_base: &Url,
    reps: impl IntoIterator<Item = &Representation>,
    pb: Option<ProgressBar>,
) -> Result<()> {
    if let Some(pb) = pb.as_ref() {
        pb.set_message("Downloading");
    }

    let futures: Vec<_> = reps
        .into_iter()
        .map(|rep| download_init(state.clone(), client, url_base, rep))
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
    client: &Client,
    url_base: &Url,
    rep: &Representation,
) -> Result<()> {
    let media_type = rep.media_type();
    if state.lock().await.downloaded_init.contains_key(&media_type) {
        return Ok(());
    }

    let url = url_base.join(&rep.segment_template.initialization_path)?;
    let resp = client.get(url.as_str()).send().await?;
    if resp.status() == StatusCode::NOT_FOUND {
        return Err(IgLiveError::StatusNotFound.into());
    }
    if !resp.status().is_success() {
        eprintln!("Received status code {}", resp.status().as_u16());
        return Err(anyhow!("Failed to download {}", url.as_str()));
    }

    let buffer: Vec<_> = resp.bytes().await?.into_iter().collect();

    state.lock().await.downloaded_init.insert(media_type, buffer);

    Ok(())
}
