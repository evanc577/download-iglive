use thiserror::Error;

#[derive(Error, Debug)]
pub enum IgtvError {
    #[error("Invalid URL")]
    InvalidUrl,
}
