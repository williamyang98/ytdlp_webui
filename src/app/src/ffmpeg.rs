use std::path::Path;
use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;
use crate::database::AudioExtension;
use youtube_api::{Thumbnail, VideoId, VideoItem, PaginatedResponse};

#[derive(Clone,Copy,Debug)]
enum SizeBytes {
    Byte,
    KiB,
    KB,
    MiB,
    MB,
    GiB,
    GB,
}

impl TryFrom<&str> for SizeBytes {
    type Error = &'static str;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        match v {
            "B"   => Ok(Self::Byte),
            "KiB" => Ok(Self::KiB),
            "KB" => Ok(Self::KB),
            "kB" => Ok(Self::KB),
            "MiB" => Ok(Self::MiB),
            "MB" => Ok(Self::MB),
            "GiB" => Ok(Self::GiB),
            "GB" => Ok(Self::GB),
            _ => Err("Unknown unit"),
        }
    }
}

impl SizeBytes {
    fn to_bytes(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::KiB => 1024,
            Self::MiB => 1024*1024,
            Self::GiB => 1024*1024*1024,
            Self::KB => 1000,
            Self::MB => 1000*1000,
            Self::GB => 1000*1000*1000,
        }
    }
}

#[derive(Clone,Copy,Debug)]
enum SizeBits {
    Bits,
    Kb,
    Mb,
    Gb,
}

impl SizeBits {
    fn try_from_long(v: &str) -> Option<Self> {
        match v {
            "bits"   => Some(Self::Bits),
            "kbits" => Some(Self::Kb),
            "Mbits" => Some(Self::Mb),
            "Gbits" => Some(Self::Gb),
            _ => None,
        }
    }

    fn try_from_short(v: &str) -> Option<Self> {
        match v {
            "b"   => Some(Self::Bits),
            "kb" => Some(Self::Kb),
            "Mb" => Some(Self::Mb),
            "Gb" => Some(Self::Gb),
            _ => None,
        }
    }
}

impl SizeBits {
    fn to_bits(self) -> usize {
        match self {
            Self::Bits => 1,
            Self::Kb => 1_000,
            Self::Mb => 1_000_000,
            Self::Gb => 1_000_000_000,
        }
    }
}

#[derive(Clone,Copy,Debug,Default)]
pub struct Time {
    pub days: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: f32,
}

impl Time {
    pub fn to_milliseconds(&self) -> u64 {
        let mut v: u64 = 0;
        v += (self.seconds*1000.0) as u64;
        v += self.minutes as u64 * 1000*60;
        v += self.hours   as u64 * 1000*60*60;
        v += self.days    as u64 * 1000*60*60*24;
        v
    }
}

#[derive(Clone,Debug,Error)]
pub enum TimeParseError {
    #[error("Failed to parse seconds: {0}")]
    InvalidSeconds(std::num::ParseFloatError),
    #[error("Failed to parse minutes: {0}")]
    InvalidMinutes(std::num::ParseIntError),
    #[error("Failed to parse hours: {0}")]
    InvalidHours(std::num::ParseIntError),
    #[error("Failed to parse days: {0}")]
    InvalidDays(std::num::ParseIntError),
}

impl Time {
    pub fn try_from_str(v: &str) -> Result<Self, TimeParseError> {
        type E = TimeParseError;
        let mut parts: Vec<&str> = v.split(':'). collect();
        parts.reverse();
        let mut time = Time::default();
        if let Some(v) = parts.first() { time.seconds = v.parse().map_err(E::InvalidSeconds)?; }
        if let Some(v) = parts.get(1) { time.minutes = v.parse().map_err(E::InvalidMinutes)?; }
        if let Some(v) = parts.get(2) { time.hours = v.parse().map_err(E::InvalidHours)?; }
        if let Some(v) = parts.get(3) { time.days = v.parse().map_err(E::InvalidDays)?; }
        Ok(time)
    }
}

const FLOAT32_REGEX: &str = r"\d+(?:\.\d+)?";
const BYTES_REGEX: &str = r"[kKMG]i?B";
const BITS_LONG_REGEX: &str = r"[kMG]?bits";
const BITS_SHORT_REGEX: &str = r"[kMG]?b";
const TIME_REGEX: &str = r"(?:\d+:)*\d+(?:\.\d+)?";

#[derive(Clone,Copy,Debug,Default)]
pub struct TranscodeProgress {
    pub frame: Option<usize>,
    pub fps: Option<f32>,
    pub q_factor: Option<f32>,
    pub size_bytes: Option<usize>,
    pub total_time_transcoded: Option<Time>,
    pub speed_bits: Option<usize>,
    pub speed_factor: Option<f32>,
}

#[derive(Clone,Copy,Debug,Default)]
pub struct TranscodeSourceInfo {
    pub duration: Option<Time>,
    pub start_time: Option<Time>,
    pub speed_bits: Option<usize>,
}

#[derive(Debug)]
pub enum ParsedStderrLine {
    TranscodeProgress(TranscodeProgress),
    TranscodeSourceInfo(TranscodeSourceInfo),
}

pub fn parse_stderr_line(line: &str) -> Option<ParsedStderrLine> {
    lazy_static! {
        static ref PROGRESS_REGEX: Regex = Regex::new(format!(
            r"(?:frame\s*=\s*(\d+)\s+fps\s*=\s*({2})\s+q\s*=\s*({2})\s+)?size\s*=\s*(\d+)({0})\s+time\s*=\s*({1})\s+bitrate\s*=\s*({2})({3})\/s\s+speed\s*=\s*({2})\s*x",
            BYTES_REGEX, TIME_REGEX, FLOAT32_REGEX, BITS_LONG_REGEX,
        ).as_str()).unwrap();
        static ref SOURCE_INFO_REGEX: Regex = Regex::new(format!(
            r"Duration:\s*({0}),\s*start:\s*({1}),\s*bitrate:\s*({2})\s*({3})\/s",
            TIME_REGEX, TIME_REGEX, FLOAT32_REGEX, BITS_SHORT_REGEX,
        ).as_str()).unwrap();
    }
    let line = line.trim();
    if let Some(captures) = PROGRESS_REGEX.captures(line) {
        let frame: Option<usize> = captures.get(1).and_then(|m| m.as_str().parse().ok());
        let fps: Option<f32> = captures.get(2).and_then(|m| m.as_str().parse().ok());
        let q_factor: Option<f32> = captures.get(3).and_then(|m| m.as_str().parse().ok());
        let size_bytes = {
            let value: Option<u32> = captures.get(4).and_then(|m| m.as_str().parse().ok());
            let unit: Option<SizeBytes> = captures.get(5).and_then(|m| m.as_str().try_into().ok());
            match (value, unit) {
                (Some(value), Some(unit)) => Some(value as usize * unit.to_bytes()),
                _ => None,
            }
        };
        let total_time_transcoded: Option<Time> = captures.get(6).and_then(|m| Time::try_from_str(m.as_str()).ok());
        let speed_bits = {
            let value: Option<f32> = captures.get(7).and_then(|m| m.as_str().parse().ok());
            let unit: Option<SizeBits> = captures.get(8).and_then(|m| SizeBits::try_from_long(m.as_str()));
            match (value, unit) {
                (Some(value), Some(unit)) => Some((value * unit.to_bits() as f32) as usize),
                _ => None,
            }
        };
        let speed_factor: Option<f32> = captures.get(9).and_then(|m| m.as_str().parse().ok());
        let result = TranscodeProgress {
            frame,
            fps,
            q_factor,
            size_bytes,
            total_time_transcoded,
            speed_bits,
            speed_factor,
        };
        return Some(ParsedStderrLine::TranscodeProgress(result));
    } else if let Some(captures) = SOURCE_INFO_REGEX.captures(line) {
        let duration: Option<Time> = captures.get(1).and_then(|m| Time::try_from_str(m.as_str()).ok());
        let start_time: Option<Time> = captures.get(2).and_then(|m| Time::try_from_str(m.as_str()).ok());
        let speed_bits = {
            let value: Option<f32> = captures.get(3).and_then(|m| m.as_str().parse().ok());
            let unit: Option<SizeBits> = captures.get(4).and_then(|m| SizeBits::try_from_short(m.as_str()));
            match (value, unit) {
                (Some(value), Some(unit)) => Some((value * unit.to_bits() as f32) as usize),
                _ => None,
            }
        };
        let result = TranscodeSourceInfo {
            duration,
            start_time,
            speed_bits,
        };
        return Some(ParsedStderrLine::TranscodeSourceInfo(result));
    }
    None
}

pub fn create_ffmpeg_transcode_arguments(
    input_path: &Path, output_path: &Path,
    video_id: &VideoId, audio_ext: AudioExtension,
    video_info: Option<&PaginatedResponse<VideoItem>>,
) -> Vec<String> {
    // spawn process
    let mut args = Vec::<String>::new();
    let push_args = |args: &mut Vec<String>, values: &[&str]| {
        args.extend(values.iter().map(|&s| s.to_owned()));
    };
    let push_video_info = |args: &mut Vec<String>, field: &str, value: &str| {
        args.extend(["-video_info".to_owned(), format!("{0}={1}", field, value)]);
    };
    push_args(&mut args, &["-i", input_path.to_str().unwrap()]);
    let can_embed_thumbnail = &[AudioExtension::MP3].contains(&audio_ext);
    let thumbnail = || -> Option<Thumbnail> {
        if !can_embed_thumbnail {
            return None;
        }
        let video_info = video_info?;
        let item = video_info.items.first()?;
        let mut thumbnails: Vec<Thumbnail> = item.snippet.thumbnails.values().cloned().collect();
        thumbnails.sort_by_key(|thumbnail| thumbnail.width * thumbnail.height);
        thumbnails.last().cloned()
    } ();
    if let Some(ref thumbnail) = thumbnail {
        push_args(&mut args, &["-i", thumbnail.url.as_str()]);
    }
    push_args(&mut args, &["-map", "0:a"]);
    if thumbnail.is_some() {
        push_args(&mut args, &["-map", "1"]);
    }
    push_video_info(&mut args, "video_id", video_id.as_str());
    if let Some(video_info) = video_info {
        if let Some(item) = video_info.items.first() {
            push_video_info(&mut args, "title", item.snippet.title.as_str());
            push_video_info(&mut args, "artist", item.snippet.channel_title.as_str());
            push_video_info(&mut args, "description", item.snippet.description.as_str());
            push_video_info(&mut args, "published_at", serde_json::to_string(&item.snippet.published_at).unwrap().as_str());
            push_args(&mut args, &["-id3v2_version", "3"]);
            let mut thumbnails: Vec<(&String, &Thumbnail)> = item.snippet.thumbnails.iter().collect();
            thumbnails.sort_by_key(|(_, thumbnail)| thumbnail.width * thumbnail.height);
        }
    }
    if thumbnail.is_some() {
        push_args(&mut args, &["-disposition:0", "attached_pic"]);
    }
    push_args(&mut args, &[
        "-threads", "0",
        "-progress", "-", "-y",
        output_path.to_str().unwrap(),
    ]);
    args
}
