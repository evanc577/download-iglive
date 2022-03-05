use std::path::Path;

use anyhow::Result;
use futures::future;
use indicatif::ProgressBar;
use reqwest::Url;

use crate::mpd::Representation;

pub async fn download_reps_forwards(
    url_base: &Url,
    reps: impl IntoIterator<Item = (&Representation, &ProgressBar)>,
    dir: impl AsRef<Path> + Send,
) -> Result<()> {
    let futures: Vec<_> = reps
        .into_iter()
        .map(|(rep, pb)| download_forwards(url_base, rep, dir.as_ref(), pb))
        .collect();
    future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<_>>()?;
    Ok(())
}

async fn download_forwards(
    url_base: &Url,
    rep: &Representation,
    dir: impl AsRef<Path>,
    pb: &ProgressBar,
) -> Result<()> {
    todo!()
}
