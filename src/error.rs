use thiserror::Error;

#[derive(Error, Debug)]
pub enum IgLiveError {
    #[error("Invalid URL")]
    InvalidUrl,
    #[error("Status code 404 received")]
    StatusNotFound,
    #[error("Missing init")]
    MissingInit,
    #[error("ffmpeg process failed")]
    FfmpegFail,
}
