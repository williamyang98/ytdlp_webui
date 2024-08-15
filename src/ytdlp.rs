use std::ffi::OsStr;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;

// NOTE: The ytdlp cli output is not stable, but we can manually format certain outputs
//       We will then do pattern matching on that controlled output
pub fn get_ytdlp_arguments<'a>(url: &'a str, ffmpeg_binary_path: &'a str, output_format: &'a str) -> impl IntoIterator<Item=impl AsRef<OsStr> + 'a> {
    [
        url,
        "--extract-audio",
        "--format", "bestaudio",
        "--no-continue", // override existing files
        "--no-simulate", // avoid running simulation when changing templates
        "--ffmpeg-location", ffmpeg_binary_path,
        // format progress string
        "--progress", "--newline",
        "--progress-template", concat!(
            "@[progress] ",
            "eta=%(progress.eta)d,elapsed=%(progress.elapsed)d,",
            "downloaded_bytes=%(progress.downloaded_bytes)d,total_bytes=%(progress.total_bytes)d,",
            "speed=%(progress.speed)d",
        ),
        "--output", output_format, // "%(id)s.%(ext)s", // detect name of audio after command runs
        "--print", "@[download-path] %(filename)s",
        "--print", "before_dl:@[before-dl-path] %(filename)s",
        "--print", "pre_process:@[pre-process-path] %(filename)s",
        "--print", "post_process:@[post-process-path] %(filename)s",
        "--print", "after_move:@[after-move-path] %(filename)s",
        "--verbose", // print extra debug info to stderr
    ]
}

#[derive(Clone,Copy,Debug,Default,Serialize)]
pub struct DownloadProgress {
    pub eta_seconds: Option<u64>,
    pub elapsed_seconds: Option<u64>,
    pub downloaded_bytes: Option<usize>,
    pub total_bytes: Option<usize>,
    pub speed_bytes: Option<usize>,
}

const YOUTUBE_ID_REGEX: &str = r"[a-zA-Z0-9\\/.\-\_]+";

#[derive(Debug)]
pub enum ParsedStdoutLine {
    DownloadProgress(DownloadProgress),
    OutputPath(String),
}

pub fn parse_stdout_line(line: &str) -> Option<ParsedStdoutLine> {
    lazy_static! {
        static ref DOWNLOAD_PROGRESS_REGEX: Regex = Regex::new(
            r"@\[progress\]\s+eta=(\d+)?,elapsed=(\d+)?,downloaded_bytes=(\d+),total_bytes=(\d+),speed=(\d+)?",
        ).unwrap();
        static ref OUTPUT_PATH_REGEX: Regex = Regex::new(format!(
            r"@\[after-move-path\]\s+({0})", YOUTUBE_ID_REGEX,
        ).as_str()).unwrap();
    }
    let line = line.trim();
    if let Some(captures) = DOWNLOAD_PROGRESS_REGEX.captures(line) {
        let eta_seconds: Option<u64> = captures.get(1).and_then(|m| m.as_str().parse().ok());
        let elapsed_seconds: Option<u64> = captures.get(2).and_then(|m| m.as_str().parse().ok());
        let downloaded_bytes: Option<usize> = captures.get(3).and_then(|m| m.as_str().parse().ok());
        let total_bytes: Option<usize> = captures.get(4).and_then(|m| m.as_str().parse().ok());
        let speed_bytes: Option<usize> = captures.get(5).and_then(|m| m.as_str().parse().ok());
        let result = DownloadProgress {
            eta_seconds,
            elapsed_seconds,
            downloaded_bytes,
            total_bytes,
            speed_bytes,
        };
        return Some(ParsedStdoutLine::DownloadProgress(result));
    }
    if let Some(captures) = OUTPUT_PATH_REGEX.captures(line) {
        let filename: Option<String> = captures.get(1).map(|m| m.as_str().to_owned());
        return Some(ParsedStdoutLine::OutputPath(filename?));
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
