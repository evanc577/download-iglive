use anyhow::Result;
use reqwest::header::HeaderName;
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Mpd {
    #[serde(rename = "Period")]
    period: Period,
    #[serde(rename = "@loapStreamId")]
    pub id: String,

    #[serde(rename = "@publishFrameTime")]
    pub start_frame: usize,

    #[serde(skip)]
    pub finished: bool,
}

#[derive(Deserialize, Debug)]
struct Period {
    #[serde(rename = "AdaptationSet")]
    adaptation_sets: Vec<AdaptationSet>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct AdaptationSet {
    #[serde(rename = "Representation")]
    representations: Vec<Representation>,
    max_width: Option<usize>,
    max_height: Option<usize>,
    max_frame_rate: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct Representation {
    #[serde(rename = "SegmentTemplate")]
    pub segment_template: SegmentTemplate,
    #[serde(rename = "@mimeType")]
    pub mime_type: String,
    #[serde(rename = "@width")]
    pub width: Option<usize>,
    #[serde(rename = "@height")]
    pub height: Option<usize>,
    #[serde(rename = "@frameRate")]
    pub frame_rate: Option<usize>,
    #[serde(rename = "@bandwidth")]
    pub bandwidth: usize,
}

#[derive(Deserialize, Debug)]
pub struct SegmentTemplate {
    #[serde(rename = "SegmentTimeline")]
    pub segment_timeline: SegmentTimeline,
    #[serde(rename = "@initialization")]
    pub initialization_path: String,
    #[serde(rename = "@media")]
    pub media_path: String,
}

#[derive(Deserialize, Debug)]
pub struct SegmentTimeline {
    #[serde(rename = "S")]
    pub segments: Vec<Segment>,
}

#[derive(Deserialize, Debug)]
pub struct Segment {
    #[serde(rename = "@t")]
    pub t: usize,
    #[serde(rename = "@d")]
    pub d: usize,
}

impl Mpd {
    pub async fn download_from_url(client: &Client, url: impl AsRef<str>) -> Result<Self> {
        let resp = client.get(url.as_ref()).send().await?;
        let headers = resp.headers().clone();
        let text = resp.text().await?;

        let mut manifest: Self = quick_xml::de::from_str(&text)?;

        if let Some(v) = headers.get(HeaderName::from_static("x-fb-video-broadcast-ended")) {
            if v.to_str()? == "1" {
                manifest.finished = true;
            }
        }

        Ok(manifest)
    }

    pub fn best_media(&self) -> (&Representation, &Representation) {
        let mut cur_video_bandwidth = 0;
        let mut cur_audio_bandwidth = 0;
        let mut ret = (None, None);
        for a in &self.period.adaptation_sets {
            for r in &a.representations {
                if r.mime_type.starts_with("video") && r.bandwidth > cur_video_bandwidth {
                    cur_video_bandwidth = r.bandwidth;
                    ret = (Some(r), ret.1);
                }
                if r.mime_type.starts_with("audio") && r.bandwidth > cur_audio_bandwidth {
                    cur_audio_bandwidth = r.bandwidth;
                    ret = (ret.0, Some(r));
                }
            }
        }
        (ret.0.unwrap(), ret.1.unwrap())
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum MediaType {
    Video,
    Audio,
    Unknown,
}

impl Representation {
    pub fn media_type(&self) -> MediaType {
        if self.mime_type.starts_with("video/") {
            MediaType::Video
        } else if self.mime_type.starts_with("audio/") {
            MediaType::Audio
        } else {
            MediaType::Unknown
        }
    }

    pub fn download_url(&self, url_base: &Url, t: impl ToString) -> Result<Url> {
        Ok(url_base.join(
            &self
                .segment_template
                .media_path
                .replace("$Time$", &t.to_string()),
        )?)
    }
}
