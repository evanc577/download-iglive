use std::path::Path;

use reqwest::Url;

use super::download_file;
use crate::mpd::Representation;

pub async fn download_initialization(url_base: &Url, rep: &Representation, dir: impl AsRef<Path>) {
    let url = url_base
        .join(&rep.segment_template.initialization_path)
        .unwrap();
    let filename = dir
        .as_ref()
        .join(url.path_segments().unwrap().rev().next().unwrap());
    download_file(&url, filename).await.unwrap();
}
