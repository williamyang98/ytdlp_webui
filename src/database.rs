use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::cast::{FromPrimitive, ToPrimitive};
use thiserror::Error;
use crate::generate_bidirectional_binding;
use crate::util::get_unix_time;

#[derive(Clone,Debug,PartialEq,Eq,Hash,Serialize)]
#[serde(transparent)]
pub struct VideoId {
    id: String,
}

#[derive(Clone,Copy,Debug,Error,Serialize)]
pub enum VideoIdError {
    #[error("Invalid length: expected={expected}, given={given}")]
    InvalidLength { expected: usize, given: usize },
    #[error("Invalid character: index={index}, char={char}")]
    InvalidCharacter { index: usize, char: char },
}

impl VideoId {
    pub fn try_new(id: &str) -> Result<Self, VideoIdError> {
        const VALID_YOUTUBE_ID_LENGTH: usize = 11;
        if id.len() != VALID_YOUTUBE_ID_LENGTH {
            return Err(VideoIdError::InvalidLength { expected: VALID_YOUTUBE_ID_LENGTH, given: id.len() });
        }
        let invalid_char = id.chars().enumerate().find(|(_,c)| !matches!(c, 'A'..='Z'|'a'..='z'|'0'..='9'|'-'|'_'));
        if let Some((index, c)) = invalid_char {
            return Err(VideoIdError::InvalidCharacter { index, char: c });
        }
        Ok(Self { id: id.to_string() })
    }

    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash,Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioExtension {
    M4A,
    AAC,
    MP3,
    WEBM,
}

generate_bidirectional_binding!(
    AudioExtension, &'static str, &str,
    (M4A, "m4a"),
    (AAC, "aac"),
    (MP3, "mp3"),
    (WEBM, "webm"),
);

impl AudioExtension {
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }
}

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,FromPrimitive,ToPrimitive)]
#[serde(rename_all = "lowercase")]
pub enum WorkerStatus {
    #[default]
    None = 0,
    Queued = 1,
    Running = 2,
    Finished = 3,
    Failed = 4,
}

impl WorkerStatus {
    pub fn is_busy(&self) -> bool {
        match self {
            WorkerStatus::Queued | WorkerStatus::Running => true,
            WorkerStatus::None | WorkerStatus::Finished | WorkerStatus::Failed => false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct YtdlpRow {
    pub video_id: VideoId,
    pub status: WorkerStatus,
    pub unix_time: u64,
    pub stdout_log_path: Option<String>,
    pub stderr_log_path: Option<String>,
    pub system_log_path: Option<String>,
    pub audio_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FfmpegRow {
    pub video_id: VideoId,
    pub audio_ext: AudioExtension,
    pub status: WorkerStatus,
    pub unix_time: u64,
    pub stdout_log_path: Option<String>,
    pub stderr_log_path: Option<String>,
    pub system_log_path: Option<String>,
    pub audio_path: Option<String>,
}

pub type DatabasePool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type DatabaseConnection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

pub fn setup_database(conn: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ytdlp (
            video_id TEXT,
            status INTEGER DEFAULT 0,
            unix_time INTEGER,
            stdout_log_path TEXT,
            stderr_log_path TEXT,
            system_log_path TEXT,
            audio_path TEXT,
            PRIMARY KEY (video_id)
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ffmpeg (
            video_id TEXT,
            audio_ext TEXT,
            status INTEGER DEFAULT 0,
            unix_time INTEGER,
            stdout_log_path TEXT,
            stderr_log_path TEXT,
            system_log_path TEXT,
            audio_path TEXT,
            PRIMARY KEY (video_id, audio_ext)
        )",
        (),
    )?;
    Ok(())
}

#[derive(Debug,Clone,Copy)]
enum WorkerTable {
    Ytdlp,
    Ffmpeg,
}

generate_bidirectional_binding!(
    WorkerTable, &'static str, &str,
    (Ytdlp, "ytdlp"),
    (Ffmpeg, "ffmpeg"),
);

// insert
pub fn insert_ytdlp_entry(
    db_conn: &DatabaseConnection, video_id: &VideoId,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ytdlp.into();
    db_conn.execute(
        format!("INSERT OR REPLACE INTO {table} (video_id, status, unix_time) VALUES (?1,?2,?3)").as_str(),
        (video_id.as_str(), WorkerStatus::Queued as u8, get_unix_time()),
    )
}

pub fn insert_ffmpeg_entry(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ffmpeg.into();
    db_conn.execute(
        format!("INSERT OR REPLACE INTO {table} (video_id, audio_ext, status, unix_time) VALUES (?1,?2,?3,?4)").as_str(),
        (video_id.as_str(), audio_ext.as_str(), WorkerStatus::Queued as u8, get_unix_time()),
    )
}

// update
pub fn update_ytdlp_entry(
    db_conn: &DatabaseConnection, entry: &YtdlpRow,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ytdlp.into();
    db_conn.execute(
        format!(
            "UPDATE {table} SET \
            unix_time=?2, status=?3, \
            stdout_log_path=?4, stderr_log_path=?5, system_log_path=?6, audio_path=?7 \
            WHERE video_id=?1"
        ).as_str(),
        params![
            entry.video_id.as_str(),
            entry.unix_time, entry.status.to_u8(), 
            entry.stdout_log_path, entry.stderr_log_path, entry.system_log_path, entry.audio_path,
        ],
    )
}

pub fn update_ffmpeg_entry(
    db_conn: &DatabaseConnection, entry: &FfmpegRow,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ffmpeg.into();
    db_conn.execute(
        format!(
            "UPDATE {table} SET \
            unix_time=?3, status=?4, stdout_log_path=?5, stderr_log_path=?6, system_log_path=?7, audio_path=?8 \
            WHERE video_id=?1 AND audio_ext=?2"
        ).as_str(),
        params![
            entry.video_id.as_str(), entry.audio_ext.as_str(),
            entry.unix_time, entry.status.to_u8(),
            entry.stdout_log_path, entry.stderr_log_path, entry.system_log_path, entry.audio_path,
        ],
    )
}

// delete
pub fn delete_ytdlp_entry(db_conn: &DatabaseConnection, video_id: &VideoId) -> Result<usize, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ytdlp.into();
    db_conn.execute(format!("DELETE FROM {table} WHERE video_id=?1").as_str(), (video_id.as_str(),))
}

pub fn delete_ffmpeg_entry(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ffmpeg.into();
    db_conn.execute(
        format!("DELETE FROM {table} WHERE video_id=?1 AND audio_ext=?2").as_str(),
        (video_id.as_str(), audio_ext.as_str()),
    )
}

// select
fn map_ytdlp_row_to_entry(row: &rusqlite::Row) -> Result<YtdlpRow, rusqlite::Error> {
    let video_id: Option<String> = row.get(0)?;
    let video_id = video_id.expect("video_id is a primary key");
    let video_id = VideoId::try_new(video_id.as_str()).expect("video_id should be valid");

    let status: Option<u8> = row.get(1)?;
    let status = status.expect("status should be present");
    let status = WorkerStatus::from_u8(status).expect("status should be valid");

    let unix_time: Option<u64> = row.get(2)?;
    let unix_time = unix_time.unwrap_or(0);

    Ok(YtdlpRow {
        video_id,
        status,
        unix_time,
        stdout_log_path: row.get(3)?,
        stderr_log_path: row.get(4)?,
        system_log_path: row.get(5)?,
        audio_path: row.get(6)?,
    })
}

pub fn select_ytdlp_entries(db_conn: &DatabaseConnection) -> Result<Vec<YtdlpRow>, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ytdlp.into();
    let mut stmt = db_conn.prepare(format!(
        "SELECT video_id, status, unix_time,\
         stdout_log_path, stderr_log_path, system_log_path, audio_path FROM {table}").as_str())?;
    let row_iter = stmt.query_map([], map_ytdlp_row_to_entry)?;
    let mut entries = Vec::<YtdlpRow>::new();
    for row in row_iter {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn select_ytdlp_entry(db_conn: &DatabaseConnection, video_id: &VideoId) -> Result<Option<YtdlpRow>, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ytdlp.into();
    let mut stmt = db_conn.prepare(format!(
        "SELECT video_id, status, unix_time, \
         stdout_log_path, stderr_log_path, system_log_path, audio_path \
         FROM {table} WHERE video_id=?1").as_str())?;
    stmt.query_row([video_id.as_str()], map_ytdlp_row_to_entry).optional()
}

fn map_ffmpeg_row_to_entry(row: &rusqlite::Row) -> Result<FfmpegRow, rusqlite::Error> {
    let video_id: Option<String> = row.get(0)?;
    let video_id = video_id.expect("video_id is a primary key");
    let video_id = VideoId::try_new(video_id.as_str()).expect("video_id should be valid");

    let audio_ext: Option<String> = row.get(1)?;
    let audio_ext = audio_ext.expect("audio_ext is a primary key");
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).expect("audio_ext should be valid");

    let status: Option<u8> = row.get(2)?;
    let status = status.expect("status should be present");
    let status = WorkerStatus::from_u8(status).expect("status should be valid");

    let unix_time: Option<u64> = row.get(3)?;
    let unix_time = unix_time.unwrap_or(0);

    Ok(FfmpegRow {
        video_id,
        audio_ext,
        status,
        unix_time,
        stdout_log_path: row.get(4)?,
        stderr_log_path: row.get(5)?,
        system_log_path: row.get(6)?,
        audio_path: row.get(7)?,
    })
}

pub fn select_ffmpeg_entries(db_conn: &DatabaseConnection) -> Result<Vec<FfmpegRow>, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ffmpeg.into();
    let mut stmt = db_conn.prepare(format!(
        "SELECT video_id, audio_ext, status, unix_time,\
         stdout_log_path, stderr_log_path, system_log_path, audio_path FROM {table}").as_str())?;

    let row_iter = stmt.query_map([], map_ffmpeg_row_to_entry)?;
    let mut entries = Vec::<FfmpegRow>::new();
    for row in row_iter {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn select_ffmpeg_entry(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension,
) -> Result<Option<FfmpegRow>, rusqlite::Error> {
    let table: &'static str = WorkerTable::Ffmpeg.into();
    let mut stmt = db_conn.prepare(format!(
        "SELECT video_id, audio_ext, status, unix_time,\
         stdout_log_path, stderr_log_path, system_log_path, audio_path \
         FROM {table} WHERE video_id=?1 AND audio_ext=?2").as_str())?;
    stmt.query_row([video_id.as_str(), audio_ext.as_str()], map_ffmpeg_row_to_entry).optional()
}

// select and update
pub fn select_and_update_ytdlp_entry<F>(
    db_conn: &DatabaseConnection, video_id: &VideoId, callback: F,
) -> Result<usize, rusqlite::Error> 
where F: FnOnce(&mut YtdlpRow)
{
    let entry = select_ytdlp_entry(db_conn, video_id)?;
    let Some(mut entry) = entry else {
        return Ok(0);
    };
    callback(&mut entry);
    update_ytdlp_entry(db_conn, &entry)
}

pub fn select_and_update_ffmpeg_entry<F>(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, callback: F,
) -> Result<usize, rusqlite::Error> 
where F: FnOnce(&mut FfmpegRow)
{
    let entry = select_ffmpeg_entry(db_conn, video_id, audio_ext)?;
    let Some(mut entry) = entry else {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    };
    callback(&mut entry);
    update_ffmpeg_entry(db_conn, &entry)
}
