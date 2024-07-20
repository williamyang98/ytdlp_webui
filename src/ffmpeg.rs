use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;

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

#[derive(Clone,Copy,Debug)]
enum SizeBits {
    Bits,
    Kb,
    Mb,
    Gb,
}

impl TryFrom<&str> for SizeBits {
    type Error = &'static str;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        match v {
            "bits"   => Ok(Self::Bits),
            "kbits" => Ok(Self::Kb),
            "Mbits" => Ok(Self::Mb),
            "Gbits" => Ok(Self::Gb),
            _ => Err("Unknown unit"),
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
    pub seconds: u8,
    pub milliseconds: f32,
}

impl Time {
    pub fn to_microseconds(&self) -> u64 {
        let mut v: u64 = 0;
        v += (self.milliseconds*1000.0) as u64;
        v += self.seconds as u64 * 1_000_000;
        v += self.minutes as u64 * 1_000_000*60;
        v += self.hours   as u64 * 1_000_000*60*60;
        v += self.days    as u64 * 1_000_000*60*60*24;
        v
    }
}

#[derive(Clone,Debug,Error)]
pub enum TimeParseError {
    #[error("Failed to parse milliseconds: {0}")]
    InvalidMilliseconds(std::num::ParseFloatError),
    #[error("Failed to parse seconds: {0}")]
    InvalidSeconds(std::num::ParseIntError),
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
        if let Some(v) = parts.first() { time.milliseconds = v.parse().map_err(E::InvalidMilliseconds)?; }
        if let Some(v) = parts.get(1) { time.seconds = v.parse().map_err(E::InvalidSeconds)?; }
        if let Some(v) = parts.get(2) { time.minutes = v.parse().map_err(E::InvalidMinutes)?; }
        if let Some(v) = parts.get(3) { time.hours = v.parse().map_err(E::InvalidHours)?; }
        if let Some(v) = parts.get(4) { time.days = v.parse().map_err(E::InvalidDays)?; }
        Ok(time)
    }
}

const FLOAT32_REGEX: &str = r"\d*[.]?\d+";
const BYTES_REGEX: &str = r"[KMG]iB";
const BITS_REGEX: &str = r"[kMG]?bits";
const TIME_REGEX: &str = r"(?:\d+:)+\d+[.]\d+";

#[derive(Clone,Copy,Debug,Default)]
pub struct TranscodeProgress {
    pub size_bytes: Option<usize>,
    pub time_elapsed: Option<Time>,
    pub speed_bits: Option<usize>,
    pub speed_factor: Option<u32>,
}

#[derive(Debug)]
pub enum ParsedStderrLine {
    TranscodeProgress(TranscodeProgress),
}

pub fn parse_stderr_line(line: &str) -> Option<ParsedStderrLine> {
    lazy_static! {
        static ref PROGRESS_REGEX: Regex = Regex::new(format!(
            r"size=\s*(\d+)({0})\s+time=\s*({1})\s+bitrate=\s*({2})({3})/s\s+speed=\s*(\d+)x",
            BYTES_REGEX, TIME_REGEX, FLOAT32_REGEX, BITS_REGEX,
        ).as_str()).unwrap();
    }
    let line = line.trim();
    if let Some(captures) = PROGRESS_REGEX.captures(line) {
        let size_bytes = {
            let value: Option<u32> = captures.get(1).and_then(|m| m.as_str().parse().ok());
            let unit: Option<SizeBytes> = captures.get(2).and_then(|m| m.as_str().try_into().ok());
            match (value, unit) {
                (Some(value), Some(unit)) => Some(value as usize * unit.to_bytes()),
                _ => None,
            }
        };
        let time_elapsed: Option<Time> = captures.get(3).and_then(|m| Time::try_from_str(m.as_str()).ok());
        let speed_bits = {
            let value: Option<f32> = captures.get(4).and_then(|m| m.as_str().parse().ok());
            let unit: Option<SizeBits> = captures.get(5).and_then(|m| m.as_str().try_into().ok());
            match (value, unit) {
                (Some(value), Some(unit)) => Some((value * unit.to_bits() as f32) as usize),
                _ => None,
            }
        };
        let speed_factor: Option<u32> = captures.get(6).and_then(|m| m.as_str().parse().ok());
        let result = TranscodeProgress {
            size_bytes,
            time_elapsed,
            speed_bits,
            speed_factor,
        };
        return Some(ParsedStderrLine::TranscodeProgress(result));
    }
    None
}
