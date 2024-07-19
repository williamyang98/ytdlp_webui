use crate::generate_bidirectional_binding;
use crate::util::get_unix_time;

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct VideoId {
    id: String,
}

#[derive(Clone,Copy,Debug)]
pub enum VideoIdError {
    InvalidLength { expected: usize, given: usize },
    InvalidCharacter(char),
}

impl VideoId {
    pub fn try_new(id: &str) -> Result<Self, VideoIdError> {
        const VALID_YOUTUBE_ID_LENGTH: usize = 11;
        if id.len() != VALID_YOUTUBE_ID_LENGTH {
            return Err(VideoIdError::InvalidLength { expected: VALID_YOUTUBE_ID_LENGTH, given: id.len() });
        }
        let invalid_char = id.chars().find(|c| !matches!(c, 'A'..='Z'|'a'..='z'|'0'..='9'|'-'|'_'));
        if let Some(c) = invalid_char {
            return Err(VideoIdError::InvalidCharacter(c));
        }
        Ok(Self { id: id.to_string() })
    }

    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
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

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum WorkerStatus {
    None,
    Queued,
    Running,
    Finished,
    Failed,
}

generate_bidirectional_binding!(
    WorkerStatus, u8, u8,
    (None, 0),
    (Queued, 1),
    (Running, 2),
    (Finished, 3),
    (Failed, 4),
);

generate_bidirectional_binding!(
    WorkerStatus, &'static str, &str,
    (None, "none"),
    (Queued, "queued"),
    (Running, "running"),
    (Finished, "finished"),
    (Failed, "failed"),
);

impl WorkerStatus {
    pub fn is_busy(&self) -> bool {
        match self {
            WorkerStatus::Queued | WorkerStatus::Running => true,
            WorkerStatus::None | WorkerStatus::Finished | WorkerStatus::Failed => false,
        }
    }
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
        (video_id.as_str(), audio_ext.as_str(), Into::<u8>::into(WorkerStatus::Queued), get_unix_time()),
    )
}

pub fn update_worker_status(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, status: WorkerStatus, table: WorkerTable,
) -> Result<usize, rusqlite::Error> {
    let table: &'static str = table.into();
    db_conn.execute(
        format!("UPDATE {table} SET status=?3, unix_time=?4 WHERE video_id=?1 AND audio_ext=?2").as_str(),
        (video_id.as_str(), audio_ext.as_str(), Into::<u8>::into(status), get_unix_time()),
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
    let Some(status) = TryFrom::<u8>::try_from(status).ok() else {
        return Err(StatusFetchError::InvalidEnumValue(status));
    };
    Ok(status)
}

pub fn select_worker_fields<T, F>(
    db_conn: &DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, table: WorkerTable, 
    fields: &[&str], transform: F,
) -> Result<T, rusqlite::Error> 
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
    select_query.query_row([video_id.as_str(), audio_ext.as_str()], transform)
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
