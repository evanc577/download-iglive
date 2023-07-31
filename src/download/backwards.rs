use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use futures::future;
use indicatif::ProgressBar;
use reqwest::{Client, Url};
use tokio::sync::Mutex;

use super::download_file;
use crate::error::IgLiveError;
use crate::mpd::Representation;
use crate::state::State;

pub async fn download_reps_backwards(
    state: Arc<Mutex<State>>,
    client: &Client,
    url_base: &Url,
    reps: impl IntoIterator<Item = (&Representation, ProgressBar)>,
    start_frame: usize,
    dir: impl AsRef<Path> + Send,
) -> Result<()> {
    let futures: Vec<_> = reps
        .into_iter()
        .map(|(rep, pb)| {
            download_backwards(
                state.clone(),
                client,
                url_base,
                rep,
                start_frame,
                dir.as_ref(),
                pb,
            )
        })
        .collect();
    future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<_>>()?;
    Ok(())
}

/// Download past segments
///
/// Since Instagram only returns the latest segments, we need to guess the segments numbers to
/// access past segments. This function uses an adaptive guessing method that tries the most common
/// time deltas before brute forcing all other deltas.
async fn download_backwards(
    state: Arc<Mutex<State>>,
    client: &Client,
    url_base: &Url,
    rep: &Representation,
    start_frame: usize,
    dir: impl AsRef<Path>,
    pb: ProgressBar,
) -> Result<()> {
    let media_type = rep.media_type();

    // Local copy
    let mut deltas = state.lock().await.deltas[&media_type].clone();

    // Get latest time
    let mut latest_t = *state.lock().await.downloaded_segs[&media_type]
        .iter()
        .min()
        .unwrap() as isize;

    // Try downloading segments until the first one is reached
    'outer: loop {
        if latest_t <= start_frame as isize {
            // If reached first frame, finish successfully
            pb.finish_with_message("Finished");
            return Ok(());
        }

        // Regenerate seed
        let mut v = Vec::from_iter(deltas.clone());
        v.sort_by(|&(_, a), &(_, b)| b.cmp(&a));
        let new_seed: Vec<_> = v.iter().map(|(d, _)| *d).collect();

        let mut lower_bound = 0;

        for x in OffsetRange::new(10, new_seed) {
            let t = latest_t - x;
            if t < lower_bound {
                continue;
            }

            // Update progress bar
            pb.set_message(format!("Downloaded segment {}, checking {}", latest_t, t));
            pb.tick();

            // Try to download segment
            let url = rep.download_url(url_base, t)?;
            let filename = dir.as_ref().join(
                url.path_segments()
                    .ok_or(IgLiveError::InvalidUrl)?
                    .rev()
                    .next()
                    .ok_or(IgLiveError::InvalidUrl)?,
            );
            let download_result = download_file(
                state.clone(),
                client,
                rep.media_type(),
                true,
                &url,
                filename,
            )
            .await;
            match download_result {
                Ok(()) => {
                    // Segment exists, continue onto next segment
                    latest_t = t;
                    // Update local copy
                    *deltas.entry(x).or_insert(0) += 1;
                    // Update global copy
                    *state
                        .lock()
                        .await
                        .deltas
                        .get_mut(&media_type)
                        .unwrap()
                        .entry(x)
                        .or_insert(0) += 1;
                    continue 'outer;
                }
                Err(e) => {
                    if let Some(e) = e.downcast_ref::<IgLiveError>() {
                        match e {
                            // 404 segment number does not exist
                            IgLiveError::StatusNotFound => continue,
                            // Segment exists but its PTS is too early, adjust the lower bound and
                            // try again
                            IgLiveError::PtsTooEarly => {
                                pb.println("Info: PTS too early");
                                lower_bound = t;
                                continue;
                            }
                            // Other download error, skip the segment
                            _ => {
                                pb.println(format!("Download failed: {e:?}"));
                                continue 'outer;
                            }
                        }
                    }
                }
            }
        }
    }
}

struct OffsetRange {
    visited: HashSet<isize>,
    max_diff: isize,
    offset: isize,
    seed: Vec<isize>,
    seed_idx: usize,
}

impl OffsetRange {
    fn new<T: IntoIterator<Item = isize>>(max_diff: isize, seed: T) -> Self {
        Self {
            visited: HashSet::new(),
            max_diff,
            offset: 0,
            seed: seed.into_iter().collect(),
            seed_idx: 0,
        }
    }
}

impl Iterator for OffsetRange {
    type Item = isize;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = loop {
            if self.seed_idx >= self.seed.len() {
                self.seed_idx = 0;
                self.offset *= -1;
                if self.offset >= 0 {
                    self.offset += 1;
                    if self.offset > self.max_diff {
                        return None;
                    }
                }
            }
            self.seed_idx += 1;

            let ret = self.seed[self.seed_idx - 1] + self.offset;
            if ret <= 0 {
                continue;
            }
            if self.visited.contains(&ret) {
                continue;
            }
            self.visited.insert(ret);
            break ret;
        };
        Some(ret)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn offset_range() {
        for x in OffsetRange::new(5, [2000, 2001, 2003]) {
            println!("{}", x);
        }
    }
}
