use thiserror::Error;

#[derive(Error, Debug)]
pub enum IgLiveError {
    #[error("Invalid URL")]
    InvalidUrl,
    #[error("Received status code 404received")]
    StatusNotFound,
    #[error("Received status code {0}, url: {1}")]
    StatusError(u16, String),
    #[error("Missing init")]
    FfmpegFail,
    #[error("PTS too early")]
    PtsTooEarly,
}
