use std::ffi::OsStr;
use std::io::prelude::*;
use std::path::Path;
use std::{fs, process};

use anyhow::Result;

use crate::error::IgLiveError;

/// Merge video and audio segments downloaded by [download][crate::download::download] into a
/// single `.mp4` video file.
/// `ffmpeg` is required in `$PATH`.
///
/// The output file will be placed in `dir`.
///
/// # Arguments
///
/// `dir` - Directory containing downloaded video and audio segments.
pub fn merge(dir: impl AsRef<Path>) -> Result<()> {
    let mut video_segments = vec![];
    let mut audio_segments = vec![];

    println!("Merging video file");

    // Read all files in output directory
    let segments_dir = dir.as_ref().join("segments");
    for entry in (fs::read_dir(segments_dir)?).flatten() {
        // Skip directories
        if entry.file_type()?.is_dir() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(".m4v") {
            video_segments.push(entry.path());
        } else if file_name.ends_with(".m4a") {
            audio_segments.push(entry.path());
        }
    }

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
    merge_segments(video_segments, &video_concat)?;
    merge_segments(audio_segments, &audio_concat)?;

    // Mux into final file
    let output_path = dir.as_ref().join(file_name_base + ".mp4");
    let output = process::Command::new("ffmpeg")
        .args([OsStr::new("-i"), video_concat.as_os_str()])
        .args([OsStr::new("-i"), audio_concat.as_os_str()])
        .args(["-c", "copy"])
        .arg("-y")
        .arg(&output_path)
        .output()?;

    // Remove concatenated files
    let _ = fs::remove_file(video_concat);
    let _ = fs::remove_file(audio_concat);

    if !output.status.success() {
        Err(IgLiveError::FfmpegFail.into())
    } else {
        println!("Merged video written to {:?}", output_path);
        Ok(())
    }
}

fn merge_segments(
    segs: impl IntoIterator<Item = impl AsRef<Path>>,
    path: impl AsRef<Path>,
) -> Result<()> {
    let mut output = fs::File::create(path.as_ref())?;

    // Write segments
    for seg in segs.into_iter() {
        output.write_all(&fs::read(seg)?)?;
    }

    Ok(())
}
