use std::path::Path;

use reqwest::Url;

use crate::mpd::Representation;

pub async fn download_forwards(url_base: &Url, rep: &Representation, dir: impl AsRef<Path>) {}
