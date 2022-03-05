use std::ffi::OsStr;
use std::io::prelude::*;
use std::path::Path;
use std::{fs, process};

use anyhow::Result;

use crate::error::IgtvError;

pub fn merge(dir: impl AsRef<Path>) -> Result<()> {
    let mut video_segments = vec![];
    let mut audio_segments = vec![];
    let mut video_init = None;
    let mut audio_init = None;

    println!("Merging video file");

    // Read all files in output directory
    let segments_dir = dir.as_ref().join("segments");
    for entry in (fs::read_dir(segments_dir)?).flatten() {
        // Skip directories
        if entry.file_type()?.is_dir() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with("init.m4v") {
            video_init = Some(entry.path());
        } else if file_name.ends_with("init.m4a") {
            audio_init = Some(entry.path());
        } else if file_name.ends_with(".m4v") {
            video_segments.push(entry.path());
        } else if file_name.ends_with(".m4a") {
            audio_segments.push(entry.path());
        }
    }

    // Check that init files exist
    let video_init = video_init.ok_or(IgtvError::MissingInit)?;
    let audio_init = audio_init.ok_or(IgtvError::MissingInit)?;

    // Sort segments
    video_segments.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));
    audio_segments.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));

    // Concatenate segments
    let file_name_base = dir
        .as_ref()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let video_concat = dir.as_ref().join(file_name_base.clone() + "video.tmp");
    let audio_concat = dir.as_ref().join(file_name_base.clone() + "audio.tmp");
    merge_segments(video_init, video_segments, &video_concat)?;
    merge_segments(audio_init, audio_segments, &audio_concat)?;

    // Mux into final file
    let output_path = dir.as_ref().join(file_name_base + ".mp4");
    let output = process::Command::new("ffmpeg")
        .args([OsStr::new("-i"), video_concat.as_os_str()])
        .args([OsStr::new("-i"), audio_concat.as_os_str()])
        .args(["-c", "copy"])
        .arg(&output_path)
        .output()?;

    // Remove concatenated files
    let _ = fs::remove_file(video_concat);
    let _ = fs::remove_file(audio_concat);

    if !output.status.success() {
        Err(IgtvError::FfmpegFail.into())
    } else {
        println!("Merged video written to {:?}", output_path);
        Ok(())
    }
}

fn merge_segments(
    init: impl AsRef<Path>,
    segs: impl IntoIterator<Item = impl AsRef<Path>>,
    path: impl AsRef<Path>,
) -> Result<()> {
    let mut output = fs::File::create(path.as_ref())?;

    // Write init
    output.write_all(&fs::read(init)?)?;

    // Write segments
    for seg in segs.into_iter() {
        output.write_all(&fs::read(seg)?)?;
    }

    Ok(())
}
