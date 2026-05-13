use crate::generate_bidirectional_binding;
use crate::util::get_unix_time;
use crate::schema::{ytdlp, ffmpeg};
use diesel::backend::Backend;
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
pub type DatabaseConnection = SqliteConnection;
pub type DatabasePoolError = r2d2::PoolError;
pub type DatabaseError = diesel::result::Error;
pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Clone,Debug,PartialEq,Eq,Serialize,AsExpression,FromSqlRow)]
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

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,FromPrimitive,ToPrimitive,AsExpression,FromSqlRow,bytemuck::NoUninit)]
#[repr(i32)]
#[serde(rename_all = "lowercase")]
#[diesel(sql_type = Integer)]
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


pub fn open_database(url: &str) -> DatabasePool {
    let manager = r2d2::ConnectionManager::<SqliteConnection>::new(url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Could not build connection pool")
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
pub fn create_database(db_conn: &mut DatabaseConnection) {
    db_conn.run_pending_migrations(MIGRATIONS).expect("Migration failed");
}

// insert
pub fn insert_ytdlp_entry(db_conn: &mut DatabaseConnection, video_id: &VideoId) -> DatabaseResult<usize> {
    use ytdlp::dsl as e;
    diesel::insert_into(e::ytdlp)
        .values((
            e::video_id.eq(video_id),
            e::status.eq(WorkerStatus::Queued),
            e::unix_time.eq(UnixTimestamp(get_unix_time())),
        ))
        .execute(db_conn)
}

pub fn insert_ffmpeg_entry(db_conn: &mut DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension) -> DatabaseResult<usize> {
    use ffmpeg::dsl as e;
    diesel::insert_into(e::ffmpeg)
        .values((
            e::video_id.eq(video_id),
            e::audio_ext.eq(audio_ext),
            e::status.eq(WorkerStatus::Queued),
            e::unix_time.eq(UnixTimestamp(get_unix_time())),
        ))
        .execute(db_conn)
}

// update
pub fn update_ytdlp_entry(db_conn: &mut DatabaseConnection, entry: &YtdlpRow) -> DatabaseResult<usize> {
    use ytdlp::dsl as e;
    diesel::update(e::ytdlp)
        .filter(e::video_id.eq(&entry.video_id))
        .set(entry)
        .execute(db_conn)
}

pub fn update_ffmpeg_entry(db_conn: &mut DatabaseConnection, entry: &FfmpegRow) -> DatabaseResult<usize> {
    use ffmpeg::dsl as e;
    diesel::update(e::ffmpeg)
        .filter(e::video_id.eq(&entry.video_id))
        .filter(e::audio_ext.eq(entry.audio_ext))
        .set(entry)
        .execute(db_conn)
}

// delete
pub fn delete_ytdlp_entry(db_conn: &mut DatabaseConnection, video_id: &VideoId) -> DatabaseResult<usize> {
    use ytdlp::dsl as e;
    diesel::delete(e::ytdlp)
        .filter(e::video_id.eq(video_id))
        .execute(db_conn)
}

pub fn delete_ffmpeg_entry(db_conn: &mut DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension) -> DatabaseResult<usize> {
    use ffmpeg::dsl as e;
    diesel::delete(e::ffmpeg)
        .filter(e::video_id.eq(video_id))
        .filter(e::audio_ext.eq(audio_ext))
        .execute(db_conn)
}

pub fn select_ytdlp_entries(db_conn: &mut DatabaseConnection) -> DatabaseResult<Vec<YtdlpRow>> {
    use ytdlp::dsl as e;
    e::ytdlp.load::<YtdlpRow>(db_conn)
}

pub fn select_ytdlp_entry(db_conn: &mut DatabaseConnection, video_id: &VideoId) -> DatabaseResult<Option<YtdlpRow>> {
    use ytdlp::dsl as e;
    e::ytdlp
        .filter(e::video_id.eq(video_id))
        .first::<YtdlpRow>(db_conn)
        .optional()
}

pub fn select_ffmpeg_entries(db_conn: &mut DatabaseConnection) -> DatabaseResult<Vec<FfmpegRow>> {
    use ffmpeg::dsl as e;
    e::ffmpeg.select(FfmpegRow::as_select())
        .load(db_conn)
}

pub fn select_ffmpeg_entry(db_conn: &mut DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension) -> DatabaseResult<Option<FfmpegRow>> {
    use ffmpeg::dsl as e;
    e::ffmpeg
        .filter(e::video_id.eq(video_id))
        .filter(e::audio_ext.eq(audio_ext))
        .first::<FfmpegRow>(db_conn)
        .optional()
}

// select and update
pub fn select_and_update_ytdlp_entry<F>(
    db_conn: &mut DatabaseConnection, video_id: &VideoId, callback: F,
) -> DatabaseResult<usize>
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
    db_conn: &mut DatabaseConnection, video_id: &VideoId, audio_ext: AudioExtension, callback: F,
) -> DatabaseResult<usize> 
where F: FnOnce(&mut FfmpegRow)
{
    let entry = select_ffmpeg_entry(db_conn, video_id, audio_ext)?;
    let Some(mut entry) = entry else {
        return Ok(0);
    };
    callback(&mut entry);
    update_ffmpeg_entry(db_conn, &entry)
}
