use crate::download::Downloader;

mod mpd;
mod download;

#[tokio::main]
async fn main() {
    Downloader::download().await;
}

