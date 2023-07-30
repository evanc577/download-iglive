use std::collections::{HashMap, HashSet};

use crate::mpd::MediaType;

pub struct State {
    pub downloaded_init: HashMap<MediaType, Vec<u8>>,

    pub downloaded_segs: HashMap<MediaType, HashSet<usize>>,

    pub deltas: HashMap<MediaType, HashMap<isize, i32>>,

    pub back_pts: HashMap<MediaType, usize>,
}

impl State {
    pub fn new() -> Self {
        let media_types = [MediaType::Video, MediaType::Audio];

        let downloaded_segs = media_types
            .iter()
            .cloned()
            .map(|t| (t, HashSet::new()))
            .collect();

        let mut default_delta = HashMap::new();
        for x in 18..=22 {
            default_delta.insert(x * 100, 1);
            default_delta.insert(x * 100 + 33, 1);
            default_delta.insert(x * 100 + 67, 1);
        }
        default_delta.insert(2000, 10);
        default_delta.insert(100, 5);

        let deltas = media_types
            .iter()
            .cloned()
            .map(|t| (t, default_delta.clone()))
            .collect();

        Self {
            downloaded_init: HashMap::new(),
            downloaded_segs,
            back_pts: HashMap::new(),
            deltas,
        }
    }
}
