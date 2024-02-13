use std::process::Stdio;

use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn get_pts(data: Vec<u8>) -> Result<(usize, usize)> {
    let mut child = Command::new("ffprobe")
        .args([
            "-v",
            "0",
            "-show_entries",
            "stream=start_pts,duration_ts",
            "-of",
            "compact=p=0:nk=1",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    tokio::spawn(async move {
        if let outer @ Err(e) = &stdin.write_all(&data).await {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                outer.as_ref().unwrap();
            }
        }
    });
    let output = child.wait_with_output().await.unwrap();
    if !output.status.success() {
        panic!("ffmpeg error");
    }
    let data = String::from_utf8(output.stdout).unwrap();
    let data = data.split_once('|').unwrap();
    let pts_start = data.0.trim().parse().unwrap();
    let pts_end = data.1.trim().parse().unwrap();
    Ok((pts_start, pts_end))
}
