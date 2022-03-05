#![warn(missing_docs)]

//! An experimental Instagram live stream (IGTV) downloader.
//!
//! IGTV stream will be downloaded live while concurrently downloading past segments with an
//! adaptive brute force method.
//! This allows full live streams to be downloaded.
//!
//! Live streams can also be downloaded for a short while after they've ended.
//! However, a valid `.mpd` link must be provided, which may be impossible to get at that point if
//! you do not have an existing link.

/// IGTV segment downloader
pub mod download;

mod error;

/// Video and audio segment merger
pub mod merge;

mod mpd;

mod state;
