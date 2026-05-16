use std::time::Duration;

use crate::generate_bidirectional_binding;
use crate::util::get_unix_time;
use crate::schema::{ytdlp, ffmpeg};
use anyhow::Context;
use derive_more::From;
use diesel::backend::Backend;
use diesel::connection::SimpleConnection;
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::expression_methods::ExpressionMethods;
use diesel::prelude::{AsChangeset, SqliteConnection};
use diesel::query_dsl::RunQueryDsl;
use diesel::query_dsl::methods::{SelectDsl, FilterDsl};
use diesel::r2d2;
use diesel::result::OptionalExtension;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::{Text, Integer, BigInt};
use diesel::{Queryable, Selectable, SelectableHelper, QueryableByName};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::cast::FromPrimitive;
use serde::Serialize;
use thiserror::Error;

pub type DatabasePool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;
pub type DatabasePoolError = r2d2::PoolError;
pub type DatabaseError = diesel::result::Error;
pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Clone,Debug,PartialEq,Eq,Serialize,AsExpression,FromSqlRow,From)]
#[serde(transparent)]
#[diesel(sql_type = BigInt)]
pub struct UnixTimestamp(u64);

impl<DB> ToSql<BigInt, DB> for UnixTimestamp where DB: Backend, i64: ToSql<BigInt, DB> {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, DB>) -> diesel::serialize::Result {
        let v: &i64 = bytemuck::cast_ref(&self.0);
        v.to_sql(out)
    }
}

impl<DB> FromSql<BigInt, DB> for UnixTimestamp where DB: Backend, i64: FromSql<BigInt, DB> {
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let v = i64::from_sql(bytes)?;
        Ok(UnixTimestamp(v as u64))
    }
}

impl UnixTimestamp {
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn current() -> Self {
        Self(get_unix_time())
    }
}

#[derive(Clone,Debug,PartialEq,Eq,Hash,Serialize,AsExpression,FromSqlRow)]
#[serde(transparent)]
#[diesel(sql_type = Text)]
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

impl<DB> ToSql<Text, DB> for VideoId where DB: Backend, str: ToSql<Text, DB> {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, DB>) -> diesel::serialize::Result {
        self.as_str().to_sql(out)
    }
}
impl<DB> FromSql<Text, DB> for VideoId where DB: Backend, *const str: FromSql<Text, DB> {
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        type A = *const str;
        let text = A::from_sql(bytes)?;
        let text = unsafe { &*text };
        let id = VideoId::try_new(text)?;
        Ok(id)
    }
}


#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash,Serialize,AsExpression,FromSqlRow)]
#[serde(rename_all = "lowercase")]
#[diesel(sql_type = Text)]
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

impl<DB> ToSql<Text, DB> for AudioExtension where DB: Backend, str: ToSql<Text, DB> {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, DB>) -> diesel::serialize::Result {
        self.as_str().to_sql(out)
    }
}

impl<DB> FromSql<Text, DB> for AudioExtension where DB: Backend, *const str: FromSql<Text, DB> {
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        type A = *const str;
        let text = A::from_sql(bytes)?;
        let text = unsafe { &*text };
        let id = AudioExtension::try_from(text)?;
        Ok(id)
    }
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct TranscodeKey {
    pub video_id: VideoId,
    pub audio_ext: AudioExtension,
}

impl TranscodeKey {
    pub fn as_str(&self) -> String {
        format!("{}.{}", self.video_id.as_str(), self.audio_ext.as_str())
    }
}

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,FromPrimitive,ToPrimitive,AsExpression,FromSqlRow,bytemuck::NoUninit)]
#[repr(i32)]
#[serde(rename_all = "lowercase")]
#[diesel(sql_type = Integer)]
pub enum WorkerStatus {
    #[default]
    Queued = 0,
    Running = 1,
    Finished = 2,
    Failed = 3,
}

impl WorkerStatus {
    pub fn is_busy(&self) -> bool {
        match self {
            WorkerStatus::Queued | WorkerStatus::Running => true,
            WorkerStatus::Finished | WorkerStatus::Failed => false,
        }
    }

    pub fn is_healthy(&self) -> bool {
        match self {
            WorkerStatus::Failed => false,
            WorkerStatus::Queued | WorkerStatus::Finished | WorkerStatus::Running => true,
        }
    }
}

#[derive(Clone,Copy,Debug,Error,Serialize)]
pub enum WorkerStatusError {
    #[error("Invalid worker status value: {0}")]
    InvalidValue(i32),
}

impl<DB> ToSql<Integer, DB> for WorkerStatus where DB: Backend, i32: ToSql<Integer, DB> {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, DB>) -> diesel::serialize::Result {
        let v: &i32 = bytemuck::cast_ref(self);
        v.to_sql(out)
    }
}
impl<DB> FromSql<Integer, DB> for WorkerStatus where DB: Backend, i32: FromSql<Integer, DB> {
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let v = i32::from_sql(bytes)?;
        let status = WorkerStatus::from_i32(v);
        let status = status.ok_or(WorkerStatusError::InvalidValue(v))?;
        Ok(status)
    }
}

#[derive(Debug, Clone, Serialize, Queryable, QueryableByName, Selectable, AsChangeset)]
#[diesel(table_name = ytdlp)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YtdlpRow {
    pub video_id: VideoId,
    pub status: WorkerStatus,
    pub unix_time: UnixTimestamp,
    pub stdout_log_path: Option<String>,
    pub stderr_log_path: Option<String>,
    pub system_log_path: Option<String>,
    pub audio_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Queryable, QueryableByName, Selectable, AsChangeset)]
#[diesel(table_name = ffmpeg)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FfmpegRow {
    pub video_id: VideoId,
    pub audio_ext: AudioExtension,
    pub status: WorkerStatus,
    pub unix_time: UnixTimestamp,
    pub stdout_log_path: Option<String>,
    pub stderr_log_path: Option<String>,
    pub system_log_path: Option<String>,
    pub audio_path: Option<String>,
}

// https://stackoverflow.com/a/57717533
// Custom connection manager that handles busy timeouts for SQLite to retry if it encounters SQLITE_BUSY
// This is because SQLite only supports a single writer and multiple readers
// Diesel by default doesn't handle this meaning it returns a "database is locked" error even with a connection pool like r2d2
#[derive(Debug)]
pub struct SqliteConnectionOptions {
    pub enable_write_ahead_logging_mode: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SqliteConnectionOptions {
    fn on_acquire(&self, db_conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        use diesel::r2d2::Error::QueryError;
        // https://sqlite.org/wal.html
        // WAL mode provides more concurrency as readers do not block writers and a writer does not block readers. Reading and writing can proceed concurrently
        if self.enable_write_ahead_logging_mode {
            db_conn
                .batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
                .map_err(QueryError)?;
        }
        // https://www.sqlite.org/pragma.html#pragma_foreign_keys
        // OFF by default for sqlite 3.6.19 and above
        if self.enable_foreign_keys {
            db_conn
                .batch_execute("PRAGMA foreign_keys = ON;")
                .map_err(QueryError)?;
        }
        // https://www.sqlite.org/pragma.html#pragma_busy_timeout
        if let Some(duration) = self.busy_timeout {
            let millis = duration.as_millis();
            db_conn
                .batch_execute(&format!("PRAGMA busy_timeout = {millis};"))
                .map_err(QueryError)?;
        }
        Ok(())
    }
}

pub struct Database {
    connection_pool: DatabasePool,
}

pub struct DatabaseConnection(r2d2::PooledConnection<r2d2::ConnectionManager<SqliteConnection>>);

impl Database {
    pub fn open(url: &str) -> anyhow::Result<Self> {
        let manager = r2d2::ConnectionManager::<SqliteConnection>::new(url);
        let connection_timeout = Duration::from_secs(30);
        let connection_pool = r2d2::Pool::builder()
            .max_size(10)
            .connection_timeout(connection_timeout)
            .connection_customizer(Box::new(SqliteConnectionOptions {
                enable_write_ahead_logging_mode: true,
                enable_foreign_keys: false,
                busy_timeout: Some(connection_timeout),
            }))
            .build(manager)
            .context("Could not build connection pool")?;
        Ok(Self {
            connection_pool,
        })
    }

    pub fn connect(&self) -> Result<DatabaseConnection, DatabasePoolError> {
        let conn = self.connection_pool.get()?;
        Ok(DatabaseConnection(conn))
    }
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
impl DatabaseConnection {
    pub fn run_pending_migrations(&mut self) {
        self.0.run_pending_migrations(MIGRATIONS).expect("Migration failed");
    }

    pub fn insert_ytdlp_entry(&mut self, video_id: &VideoId) -> DatabaseResult<bool> {
        use ytdlp::dsl as e;
        let total = diesel::replace_into(e::ytdlp)
            .values((
                e::video_id.eq(video_id),
                e::status.eq(WorkerStatus::default()),
                e::unix_time.eq(UnixTimestamp::current()),
            ))
            .execute(&mut self.0)?;
        if total != 1 {
            log::warn!("insert_ytdlp_entry inserted {total} entries instead of 1");
        }
        Ok(total >= 1)
    }

    pub fn insert_ffmpeg_entry(&mut self, key: &TranscodeKey) -> DatabaseResult<bool> {
        use ffmpeg::dsl as e;
        let total = diesel::replace_into(e::ffmpeg)
            .values((
                e::video_id.eq(&key.video_id),
                e::audio_ext.eq(key.audio_ext),
                e::status.eq(WorkerStatus::default()),
                e::unix_time.eq(UnixTimestamp::current()),
            ))
            .execute(&mut self.0)?;
        if total != 1 {
            log::warn!("insert_ffmpeg_entry inserted {total} entries instead of 1");
        }
        Ok(total >= 1)
    }

    // update
    pub fn update_ytdlp_entry(&mut self, entry: &YtdlpRow) -> DatabaseResult<usize> {
        use ytdlp::dsl as e;
        diesel::update(e::ytdlp)
            .filter(e::video_id.eq(&entry.video_id))
            .set(entry)
            .execute(&mut self.0)
    }

    pub fn update_ffmpeg_entry(&mut self, entry: &FfmpegRow) -> DatabaseResult<usize> {
        use ffmpeg::dsl as e;
        diesel::update(e::ffmpeg)
            .filter(e::video_id.eq(&entry.video_id))
            .filter(e::audio_ext.eq(entry.audio_ext))
            .set(entry)
            .execute(&mut self.0)
    }

    // delete
    pub fn delete_ytdlp_entry(&mut self, video_id: &VideoId) -> DatabaseResult<usize> {
        use ytdlp::dsl as e;
        diesel::delete(e::ytdlp)
            .filter(e::video_id.eq(video_id))
            .execute(&mut self.0)
    }

    pub fn delete_ffmpeg_entry(&mut self, key: &TranscodeKey) -> DatabaseResult<usize> {
        use ffmpeg::dsl as e;
        diesel::delete(e::ffmpeg)
            .filter(e::video_id.eq(&key.video_id))
            .filter(e::audio_ext.eq(key.audio_ext))
            .execute(&mut self.0)
    }

    pub fn select_ytdlp_entries(&mut self) -> DatabaseResult<Vec<YtdlpRow>> {
        use ytdlp::dsl as e;
        e::ytdlp.load::<YtdlpRow>(&mut self.0)
    }

    pub fn select_ytdlp_entry(&mut self, video_id: &VideoId) -> DatabaseResult<Option<YtdlpRow>> {
        use ytdlp::dsl as e;
        e::ytdlp
            .filter(e::video_id.eq(video_id))
            .first::<YtdlpRow>(&mut self.0)
            .optional()
    }

    pub fn select_ffmpeg_entries(&mut self) -> DatabaseResult<Vec<FfmpegRow>> {
        use ffmpeg::dsl as e;
        e::ffmpeg.select(FfmpegRow::as_select())
            .load(&mut self.0)
    }

    pub fn select_ffmpeg_entry(&mut self, key: &TranscodeKey) -> DatabaseResult<Option<FfmpegRow>> {
        use ffmpeg::dsl as e;
        e::ffmpeg
            .filter(e::video_id.eq(&key.video_id))
            .filter(e::audio_ext.eq(key.audio_ext))
            .first::<FfmpegRow>(&mut self.0)
            .optional()
    }

    // select and update
    pub fn select_and_update_ytdlp_entry<F>(&mut self, video_id: &VideoId, callback: F) -> DatabaseResult<usize>
    where F: FnOnce(&mut YtdlpRow)
    {
        let entry = self.select_ytdlp_entry(video_id)?;
        let Some(mut entry) = entry else {
            return Err(DatabaseError::NotFound);
        };
        callback(&mut entry);
        self.update_ytdlp_entry(&entry)
    }

    pub fn select_and_update_ffmpeg_entry<F>(&mut self, key: &TranscodeKey, callback: F) -> DatabaseResult<usize>
    where F: FnOnce(&mut FfmpegRow)
    {
        let entry = self.select_ffmpeg_entry(key)?;
        let Some(mut entry) = entry else {
            return Err(DatabaseError::NotFound);
        };
        callback(&mut entry);
        self.update_ffmpeg_entry(&entry)
    }
}
