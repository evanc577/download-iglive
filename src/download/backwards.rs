use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use indicatif::ProgressBar;
use reqwest::Url;

use super::download_file;
use crate::mpd::Representation;

/// Download past segments
///
/// Since Instagram only returns the latest segments, we need to guess the segments numbers to
/// access past segments. This function uses an adaptive guessing method that tries the most common
/// time deltas before brute forcing all other deltas.
pub async fn download_backwards(
    url_base: &Url,
    rep: &Representation,
    dir: impl AsRef<Path>,
    pb: &ProgressBar,
) {
    // Generate initial delta guesses
    let mut seed: Vec<_> = rep
        .segment_template
        .segment_timeline
        .segments
        .iter()
        .map(|s| s.d as isize)
        .collect();
    for x in 1..=19 {
        seed.insert(0, x * 100);
        seed.insert(0, x * 100 + 33);
        seed.insert(0, x * 100 + 66);
    }
    seed.insert(0, 2000);
    let mut times = BTreeMap::new();
    for &d in &seed {
        *times.entry(d).or_insert(0) += 1;
    }

    // Get latest time
    let first_t = rep
        .segment_template
        .segment_timeline
        .segments
        .iter()
        .rev()
        .next()
        .unwrap()
        .t as isize;
    let mut latest_t = first_t;

    // Try downloading segments until the first one is rached
    'outer: loop {
        // Regenerate seed
        let mut v = Vec::from_iter(times.clone());
        v.sort_by(|&(_, a), &(_, b)| b.cmp(&a));
        let new_seed: Vec<_> = v.iter().map(|(d, _)| *d).collect();

        for x in OffsetRange::new(2000, new_seed) {
            let t = latest_t - x;
            if t < 0 {
                continue;
            }

            // Update progress bar
            pb.set_message(format!("Downloaded segment {}, checking {}", latest_t, t));
            pb.tick();

            // Try to download segment
            let url = url_base
                .join(
                    &rep.segment_template
                        .media_path
                        .replace("$Time$", &t.to_string()),
                )
                .unwrap();
            let filename = dir
                .as_ref()
                .join(url.path_segments().unwrap().rev().next().unwrap());
            match download_file(&url, filename).await {
                Ok(()) => {
                    // If file exists, continue onto next segment
                    latest_t = t;
                    *times.entry(x).or_insert(0) += 1;
                    continue 'outer;
                }
                Err(()) => (),
            }
        }
        return;
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
