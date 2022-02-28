use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;

use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use reqwest::Url;
use tokio::join;

use crate::mpd::{Mpd, Representation};

pub struct Downloader {
    t: usize,
}

impl Downloader {
    pub async fn download() {
        // Download manifest
        let url = std::env::args().nth(1).unwrap();
        let manifest = Mpd::from_url(&url).await;
        let (video_rep, audio_rep) = &manifest.best_media();

        // Create directory
        let dir_name = &manifest.id;
        fs::create_dir_all(dir_name).unwrap();

        let url_base = Url::parse(&url).unwrap();
        println!("Downloading initialization");
        let ((), ()) = join!(
            download_initialization(&url_base, video_rep, dir_name),
            download_initialization(&url_base, audio_rep, dir_name),
        );

        println!("Downloading previous segments");

        // Progressbar
        let m = MultiProgress::new();
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.fg.green} {spinner} {wide_msg}")
            .unwrap();
        let pb_video = m.add(ProgressBar::new_spinner());
        pb_video.set_style(spinner_style.clone());
        pb_video.set_prefix("Video:");
        let pb_audio = m.add(ProgressBar::new_spinner());
        pb_audio.set_style(spinner_style.clone());
        pb_audio.set_prefix("Audio:");

        let ((), ()) = join!(
            download_backwards(&url_base, video_rep, dir_name, &pb_video),
            download_backwards(&url_base, audio_rep, dir_name, &pb_audio)
        );
    }
}

async fn download_initialization(url_base: &Url, rep: &Representation, dir: impl AsRef<Path>) {
    let url = url_base
        .join(&rep.segment_template.initialization_path)
        .unwrap();
    let filename = dir
        .as_ref()
        .join(url.path_segments().unwrap().rev().next().unwrap());
    download_file(&url, filename).await.unwrap();
}

async fn download_forwards(url_base: &Url, rep: &Representation, dir: impl AsRef<Path>) {}

async fn download_backwards(url_base: &Url, rep: &Representation, dir: impl AsRef<Path>, pb: &ProgressBar) {
    let mut seed: Vec<_> = rep
        .segment_template
        .segment_timeline
        .segments
        .iter()
        .map(|s| s.d as isize)
        .collect();
    let first_t = rep
        .segment_template
        .segment_timeline
        .segments
        .iter()
        .rev()
        .next()
        .unwrap()
        .t as isize;

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

    let mut latest_t = first_t;
    'outer: loop {
        let mut v = Vec::from_iter(times.clone());
        v.sort_by(|&(_, a), &(_, b)| b.cmp(&a));
        let new_seed: Vec<_> = v.iter().map(|(d, _)| *d).collect();

        for x in OffsetRange::new(2000, &new_seed) {
            let t = latest_t - x;
            if t < 0 {
                return;
            }

            pb.set_message(format!("Downloaded segment {}, checking {}", latest_t, t));
            pb.tick();

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

async fn download_file(url: &Url, path: impl AsRef<Path>) -> Result<(), ()> {
    let resp = reqwest::get(url.as_str()).await.unwrap();
    if !resp.status().is_success() {
        return Err(());
    }
    // println!("  {}", url.as_str());
    let mut buffer = fs::File::create(path).unwrap();
    buffer.write_all(&resp.bytes().await.unwrap()).unwrap();
    Ok(())
}

struct OffsetRange {
    visited: HashSet<isize>,
    max_diff: isize,
    offset: isize,
    seed: Vec<isize>,
    seed_idx: usize,
}

impl OffsetRange {
    fn new(max_diff: isize, seed: &[isize]) -> Self {
        let mut new_seed = seed.to_vec();
        new_seed.insert(0, 1866);
        new_seed.insert(0, 1634);
        new_seed.insert(0, 1967);
        new_seed.insert(0, 2000);
        Self {
            visited: HashSet::new(),
            max_diff,
            offset: 0,
            seed: seed.to_vec(),
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
        for x in OffsetRange::new(5, &[2000, 2001, 2003]) {
            println!("{}", x);
        }
    }
}
