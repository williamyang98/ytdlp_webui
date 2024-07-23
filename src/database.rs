use rusqlite::OptionalExtension;
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
    pub audio_ext: AudioExtension,
    pub status: WorkerStatus,
    pub unix_time: u64,
    pub infojson_path: Option<String>,
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
            audio_ext TEXT,
            status INTEGER DEFAULT 0,
            unix_time INTEGER,
            infojson_path TEXT,
            stdout_log_path TEXT,
            stderr_log_path TEXT,
            system_log_path TEXT,
            audio_path TEXT,
            PRIMARY KEY (video_id, audio_ext)
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
pub enum WorkerTable {
    YTDLP,
    FFMPEG,
}

generate_bidirectional_binding!(
    WorkerTable, &'static str, &str,
    (YTDLP, "ytdlp"),
    (FFMPEG, "ffmpeg"),
);

pub fn insert_worker_entry(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, table: WorkerTable,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = table.into();
    db_conn.execute(
        format!("INSERT OR REPLACE INTO {table} (video_id, audio_ext, status, unix_time) VALUES (?1,?2,?3,?4)").as_str(),
        (video_id.as_str(), audio_ext.as_str(), WorkerStatus::Queued as u8, get_unix_time()),
    )
}

pub fn update_worker_status(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, status: WorkerStatus, table: WorkerTable,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = table.into();
    db_conn.execute(
        format!("UPDATE {table} SET status=?3, unix_time=?4 WHERE video_id=?1 AND audio_ext=?2").as_str(),
        (video_id.as_str(), audio_ext.as_str(), status.to_u8(), get_unix_time()),
    )
}

pub fn update_worker_fields(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension,
    table: WorkerTable, fields: &[&str], values: &[&(dyn rusqlite::ToSql)],
) -> Result<usize, rusqlite::Error> {
    use std::fmt::Write;
    let mut query_string = String::with_capacity(128);
    let table: &'static str = table.into();
    write!(&mut query_string, "UPDATE {table} SET ").unwrap();
    for (i, field) in fields.iter().enumerate() {
        write!(&mut query_string, "{0}=?{1}", field, i+3).unwrap();
        if i < (fields.len()-1) {
            query_string.push(',');
        }
    }
    write!(&mut query_string, " WHERE video_id=?1 AND audio_ext=?2").unwrap();

    let mut params = Vec::<Box::<dyn rusqlite::ToSql>>::with_capacity(2 + values.len());
    params.push(Box::new(video_id.as_str()));
    params.push(Box::new(audio_ext.as_str()));
    for v in values {
        params.push(Box::new(*v));
    }
    db_conn.execute(query_string.as_str(), rusqlite::params_from_iter(params.iter()))
}

#[derive(Debug)]
pub enum StatusFetchError {
    DatabasePrepare(rusqlite::Error),
    DatabaseQuery(rusqlite::Error),
    InvalidEnumValue(u8),
    MissingValue,
}

pub fn select_worker_status(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, table: WorkerTable,
) -> Result<WorkerStatus, StatusFetchError> {
    let table: &'static str = table.into();
    let mut select_query = db_conn.prepare(format!("SELECT status FROM {table} WHERE video_id=?1 AND audio_ext=?2").as_str())
        .map_err(StatusFetchError::DatabasePrepare)?;
    let status: Option<u8> = select_query.query_row([video_id.as_str(), audio_ext.as_str()], |row| row.get(0))
        .map_err(StatusFetchError::DatabaseQuery)?;
    let Some(status) = status else {
        return Err(StatusFetchError::MissingValue); 
    };
    let Some(status) = WorkerStatus::from_u8(status) else {
        return Err(StatusFetchError::InvalidEnumValue(status));
    };
    Ok(status)
}

pub fn select_worker_fields<T, F>(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, table: WorkerTable, 
    fields: &[&str], transform: F,
) -> Result<Option<T>, rusqlite::Error> 
where F: FnOnce(&rusqlite::Row<'_>) -> Result<T, rusqlite::Error>,
{
    use std::fmt::Write;
    let table: &'static str = table.into();
    let mut query = String::with_capacity(128);
    query.push_str("SELECT ");
    for (i, field) in fields.iter().enumerate() {
        query.push_str(field);
        if i < (fields.len()-1) {
            query.push(',');
        }
    }
    write!(&mut query, " FROM {table} WHERE video_id=?1 AND audio_ext=?2").expect("Query builder shouldn't fail");
    let mut select_query = db_conn.prepare(query.as_str())?;
    select_query.query_row([video_id.as_str(), audio_ext.as_str()], transform).optional()
}

pub fn delete_worker_entry(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, table: WorkerTable,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = table.into();
    db_conn.execute(
        format!("DELETE FROM {table} WHERE video_id=?1 AND audio_ext=?2").as_str(),
        (video_id.as_str(), audio_ext.as_str()),
    )
}

fn map_ytdlp_row_to_entry(row: &rusqlite::Row) -> Result<YtdlpRow, rusqlite::Error> {
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

    Ok(YtdlpRow {
        video_id,
        audio_ext,
        status,
        unix_time,
        infojson_path: row.get(4)?,
        stdout_log_path: row.get(5)?,
        stderr_log_path: row.get(6)?,
        system_log_path: row.get(7)?,
        audio_path: row.get(8)?,
    })
}

pub fn select_ytdlp_entries(db_conn: &DatabaseConnection) -> Result<Vec<YtdlpRow>, rusqlite::Error> {
    let table: &'static str = WorkerTable::YTDLP.into();
    let mut stmt = db_conn.prepare(format!(
        "SELECT video_id, audio_ext, status, unix_time, infojson_path,\
         stdout_log_path, stderr_log_path, system_log_path, audio_path FROM {table}").as_str())?;
    let row_iter = stmt.query_map([], map_ytdlp_row_to_entry)?;
    let mut entries = Vec::<YtdlpRow>::new();
    for row in row_iter {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn select_ytdlp_entry(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension,
) -> Result<Option<YtdlpRow>, rusqlite::Error> {
    let table: &'static str = WorkerTable::YTDLP.into();
    let mut stmt = db_conn.prepare(format!(
        "SELECT video_id, audio_ext, status, unix_time, infojson_path, \
         stdout_log_path, stderr_log_path, system_log_path, audio_path \
         FROM {table} WHERE video_id=?1 AND audio_ext=?2").as_str())?;
    stmt.query_row([video_id.as_str(), audio_ext.as_str()], map_ytdlp_row_to_entry).optional()
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
    let table: &'static str = WorkerTable::FFMPEG.into();
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
    let table: &'static str = WorkerTable::FFMPEG.into();
    let mut stmt = db_conn.prepare(format!(
        "SELECT video_id, audio_ext, status, unix_time,\
         stdout_log_path, stderr_log_path, system_log_path, audio_path \
         FROM {table} WHERE video_id=?1 AND audio_ext=?2").as_str())?;
    stmt.query_row([video_id.as_str(), audio_ext.as_str()], map_ffmpeg_row_to_entry).optional()
}
