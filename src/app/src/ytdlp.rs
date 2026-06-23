use std::path::{Path, PathBuf};
use std::process::Command;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use youtube_api::VideoId;

#[derive(Debug)]
pub struct Ytdlp {
    command: PathBuf,
}

impl Ytdlp {
    pub fn new(command: PathBuf) -> Self {
        Self { command }
    }

    pub fn update(&mut self) -> std::io::Result<String> {
        let output = Command::new(self.command.as_os_str())
            .arg("--verbose")
            .arg("--update")
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = stdout.to_string();
        Ok(result)
    }

    pub fn create_download_command(&self, video_id: &VideoId, output_dirpath: &Path, current_working_directory: &Path, ffmpeg_location: &Path) -> Command {
        let youtube_url = format!("https://www.youtube.com/watch?v={0}", video_id.as_str());
        // Can't canonicalize since path doesn't exist yet
        let output_filepath = output_dirpath.join("%(id)s.%(ext)s");
        let mut command = Command::new(self.command.as_os_str());
        command
            .current_dir(current_working_directory)
            .arg(youtube_url)
            .arg("--extract-audio") // output location is printed in stderr as [ExtractAudio] Destination: ...
            .arg("--verbose") // needed to print output location for --extract-audio
            .arg("--embed-chapters") // include youtube chapters
            .args(["--format", "bestaudio"])
            .arg("--no-continue") // override existing files
            .arg("--no-simulate") // avoid running simulation when changing templates
            // format progress string
            .arg("--newline")
            .arg("--progress")
            .args([
                "--progress-template", concat!(
                    "@[progress] ",
                    "eta=%(progress.eta)d,elapsed=%(progress.elapsed)d,",
                    "downloaded_bytes=%(progress.downloaded_bytes)d,total_bytes=%(progress.total_bytes)d,",
                    "speed=%(progress.speed)d",
                ),
                "--print", "@[download-path] %(filename)s",
                "--print", "before_dl:@[before-dl-path] %(filename)s",
                "--print", "pre_process:@[pre-process-path] %(filename)s",
                "--print", "post_process:@[post-process-path] %(filename)s",
                "--print", "after_move:@[after-move-path] %(filename)s",
            ])
            // filepaths
            .arg("--ffmpeg-location")
            .arg(ffmpeg_location)
            .arg("--output")
            .arg(output_filepath.as_os_str());
        command
    }

    pub fn get_version(&self) -> std::io::Result<String> {
        let output = Command::new(self.command.as_os_str())
            .arg("--version")
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = stdout.to_string();
        Ok(version)
    }
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
        static ref OUTPUT_PATH_REGEX: Regex = Regex::new(
            r"@\[after-move-path\]\s+(.+)",
        ).unwrap();
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
    ExtractPath(String),
    VideoUnavailableError { video_id: VideoId, message: String },
    AgeRestrictedError { video_id: VideoId, message: String },
    UnhandledError { video_id: VideoId, message: String },
}

pub fn parse_stderr_line(line: &str) -> Option<ParsedStderrLine> {
    lazy_static! {
        static ref USAGE_ERROR_REGEX: Regex = Regex::new(
            r"yt-dlp.exe:\s+error:\s+(.+)"
        ).unwrap();
        static ref EXTRACT_PATH_REGEX: Regex = Regex::new(
            r"\[ExtractAudio\]\s*Destination:\s*(.+)",
        ).unwrap();
        static ref RUNTIME_ERROR_REGEX: Regex = Regex::new(format!(
            r"ERROR:\s+\[youtube\]\s+({0}):\s*(.+)",
            YOUTUBE_ID_REGEX,
        ).as_str()).unwrap();
    }
    let line = line.trim();
    if let Some(captures) = USAGE_ERROR_REGEX.captures(line) {
        if let Some(error) = captures.get(1).map(|m| m.as_str()) {
            return Some(ParsedStderrLine::UsageError(error.to_owned()));
        }
    }
    if let Some(captures) = EXTRACT_PATH_REGEX.captures(line) {
        if let Some(id) = captures.get(1).map(|m| m.as_str()) {
            return Some(ParsedStderrLine::ExtractPath(id.to_owned()));
        }
    }

    let captures = RUNTIME_ERROR_REGEX.captures(line)?;
    let video_id: VideoId = captures.get(1).and_then(|m| m.as_str().try_into().ok())?;
    let error_message = captures.get(2).map(|m| m.as_str())?;

    if let Some(message) = error_message.strip_prefix("Video unavailable.") {
        let message = message.trim().to_owned();
        return Some(ParsedStderrLine::VideoUnavailableError { video_id, message });
    }

    if let Some(message) = error_message.strip_prefix("Sign in to confirm your age. This video may be inappropriate for some users.") {
        let message = message.trim().to_owned();
        return Some(ParsedStderrLine::AgeRestrictedError { video_id, message });
    }

    let error_message = error_message.trim().to_owned();
    Some(ParsedStderrLine::UnhandledError { video_id, message: error_message })
}
