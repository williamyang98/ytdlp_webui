use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone,Copy,Debug)]
enum SizeBytes {
    Byte,
    KiB,
    MiB,
    GiB,
}

impl TryFrom<&str> for SizeBytes {
    type Error = &'static str;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        match v {
            "B"   => Ok(Self::Byte),
            "KiB" => Ok(Self::KiB),
            "MiB" => Ok(Self::MiB),
            "GiB" => Ok(Self::GiB),
            _ => Err("Unknown unit"),
        }
    }
}

impl SizeBytes {
    fn to_bytes(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::KiB => 1_000,
            Self::MiB => 1_000_000,
            Self::GiB => 1_000_000_000,
        }
    }
}

#[derive(Clone,Copy,Debug,Default)]
pub struct Eta {
    pub days: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

#[derive(Clone,Debug)]
pub enum EtaParseError {
    InvalidSeconds(std::num::ParseIntError),
    InvalidMinutes(std::num::ParseIntError),
    InvalidHours(std::num::ParseIntError),
    InvalidDays(std::num::ParseIntError),
}

impl Eta {
    pub fn try_from_str(v: &str) -> Result<Self, EtaParseError> {
        type E = EtaParseError;
        let mut parts: Vec<&str> = v.split(":").collect();
        parts.reverse();
        let mut eta = Eta::default();
        if let Some(v) = parts.get(0) { eta.seconds = v.parse().map_err(E::InvalidSeconds)?; }
        if let Some(v) = parts.get(1) { eta.minutes = v.parse().map_err(E::InvalidMinutes)?; }
        if let Some(v) = parts.get(2) { eta.hours = v.parse().map_err(E::InvalidHours)?; }
        if let Some(v) = parts.get(3) { eta.days = v.parse().map_err(E::InvalidDays)?; }
        Ok(eta)
    }
}

#[derive(Clone,Copy,Debug,Default)]
pub struct DownloadProgress {
    pub percentage: Option<f32>,
    pub size_bytes: Option<usize>,
    pub speed_bytes: Option<usize>,
    pub eta: Option<Eta>,
}

const YOUTUBE_ID_REGEX: &'static str = r"[a-zA-Z0-9\\/.\-\_]+";
const FLOAT32_REGEX: &'static str = r"\d*[.]?\d+";
const UNIT_REGEX: &'static str = r"[KMG]iB";
const ETA_REGEX: &'static str = r"[0-9:]+";

#[derive(Debug)]
pub enum ParsedStdoutLine {
    DownloadProgress(DownloadProgress),
    InfoJsonPath(String),
}

pub fn parse_stdout_line(line: &str) -> Option<ParsedStdoutLine> {
    lazy_static! {
        static ref DOWNLOAD_PROGRESS_REGEX: Regex = Regex::new(format!(
            r"\[download\]\s+({0})%\s+of\s+({0})({1})\s+at\s+({0})({1})/s\s+ETA\s+({2})", 
            FLOAT32_REGEX, UNIT_REGEX, ETA_REGEX,
        ).as_str()).unwrap();
        static ref INFOJSON_REGEX: Regex = Regex::new(format!(
            r"\[info\]\s+Writing video metadata as JSON to:\s+({0})", 
            YOUTUBE_ID_REGEX,
        ).as_str()).unwrap();
    }
    let line = line.trim();
    if let Some(captures) = DOWNLOAD_PROGRESS_REGEX.captures(line) {
        let percentage: Option<f32> = captures.get(1).map(|m| m.as_str().parse().ok()).flatten();
        let size_bytes = {
            let value: Option<f32> = captures.get(2).map(|m| m.as_str().parse().ok()).flatten();
            let unit: Option<SizeBytes> = captures.get(3).map(|m| m.as_str().try_into().ok()).flatten();
            match (value, unit) {
                (Some(value), Some(unit)) => Some((value * unit.to_bytes() as f32) as usize),
                _ => None,
            }
        };
        let speed_bytes = {
            let value: Option<f32> = captures.get(4).map(|m| m.as_str().parse().ok()).flatten();
            let unit: Option<SizeBytes> = captures.get(5).map(|m| m.as_str().try_into().ok()).flatten();
            match (value, unit) {
                (Some(value), Some(unit)) => Some((value * unit.to_bytes() as f32) as usize),
                _ => None,
            }
        };
        let eta: Option<Eta> = captures.get(6).map(|m| Eta::try_from_str(m.as_str()).ok()).flatten();
        let result = DownloadProgress {
            percentage,
            size_bytes,
            speed_bytes,
            eta,
        };
        return Some(ParsedStdoutLine::DownloadProgress(result));
    }
    if let Some(captures) = INFOJSON_REGEX.captures(line) {
        if let Some(infojson_path) = captures.get(1).map(|m| m.as_str()) {
            return Some(ParsedStdoutLine::InfoJsonPath(infojson_path.to_owned()));
        }
    }
    None
}

#[derive(Clone,Debug)]
pub enum ParsedStderrLine {
    UsageError(String),
    MissingVideo(String),
}

pub fn parse_stderr_line(line: &str) -> Option<ParsedStderrLine> {
    lazy_static! {
        static ref USAGE_ERROR_REGEX: Regex = Regex::new(
            r"yt-dlp.exe:\s+error:\s+(.+)"
        ).unwrap();
        static ref MISSING_VIDEO_REGEX: Regex = Regex::new(format!(
            r"ERROR:\s+\[youtube\]\s+({0}): Video unavailable", 
            YOUTUBE_ID_REGEX,
        ).as_str()).unwrap();
    }
    let line = line.trim();
    if let Some(captures) = USAGE_ERROR_REGEX.captures(line) {
        if let Some(error) = captures.get(1).map(|m| m.as_str()) {
            return Some(ParsedStderrLine::UsageError(error.to_owned()));
        }
    }
    if let Some(captures) = MISSING_VIDEO_REGEX.captures(line) {
        if let Some(id) = captures.get(1).map(|m| m.as_str()) {
            return Some(ParsedStderrLine::MissingVideo(id.to_owned()));
        }
    }
    None
}
