use crate::download::Downloader;

mod mpd;
mod download;

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).unwrap();
    Downloader::download(&url).await;
}

