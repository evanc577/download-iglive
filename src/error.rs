use thiserror::Error;

#[derive(Error, Debug)]
pub enum IgtvError {
    #[error("Invalid URL")]
    InvalidUrl,
    #[error("Missing init")]
    MissingInit,
    #[error("ffmpeg process failed")]
    FfmpegFail,
}
