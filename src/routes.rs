use std::path::PathBuf;
use std::sync::Arc;
use actix_web::{
    error, 
    http::{header::{ContentDisposition, ContentType, DispositionParam, DispositionType}, StatusCode}, 
    web, HttpRequest, HttpResponse
};
use serde::{Deserialize, Serialize};
use derive_more::Display;
use crate::database::{
    VideoId, VideoIdError, AudioExtension, WorkerStatus,
    delete_ffmpeg_entry, select_ffmpeg_entries, select_ffmpeg_entry,
    delete_ytdlp_entry, select_ytdlp_entries, select_ytdlp_entry,
};
use crate::metadata::{get_metadata_url, MetadataCache, Metadata};
use crate::worker_download::{try_start_download_worker, DownloadState};
use crate::worker_transcode::{try_start_transcode_worker, TranscodeState, TranscodeKey};
use crate::app::AppState;

#[derive(Debug,Clone,Serialize,Display)]
#[display(fmt = "UserApiError({},{})", error, status_code)]
struct ApiError {
    error: String,
    #[serde(skip)]
    status_code: StatusCode,
}

impl ApiError {
    fn _new(error: String, status_code: StatusCode) -> Self {
        Self { error, status_code }
    }

    fn invalid_video_id(id: String, err: VideoIdError) -> Self {
        Self {
            error: format!("invalid video id {id}: {err:?}"),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    fn invalid_audio_extension(ext: String) -> Self {
        Self {
            error: format!("invalid audio extension: {ext}"),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    fn internal_server(err: impl std::fmt::Debug) -> Self {
        Self {
            error: format!("internal server error: {err:?}"),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl actix_web::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }

    fn status_code(&self) -> StatusCode {
        self.status_code 
    }
}

#[derive(Debug,Default,Clone,Serialize)]
struct RequestTranscodeResponse {
    download_status: WorkerStatus,
    transcode_status: WorkerStatus,
    is_skip_transcode: bool,
}

#[actix_web::get("/request_transcode/{video_id}/{extension}")]
#[allow(clippy::field_reassign_with_default)]
pub async fn request_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let app = req.app_data::<AppState>().unwrap().clone();
    // download audio file
    let mut response = RequestTranscodeResponse::default();
    response.download_status = try_start_download_worker(
        video_id.clone(),
        app.download_cache.clone(), app.app_config.clone(), app.db_pool.clone(), app.worker_thread_pool.clone(),
    ).map_err(ApiError::internal_server)?;
    // transcode
    let metadata = get_metadata_from_cache(video_id, app.metadata_cache).await.ok();
    response.transcode_status = try_start_transcode_worker(
        transcode_key.clone(),
        app.download_cache, app.transcode_cache, app.app_config.clone(), app.db_pool.clone(), app.worker_thread_pool.clone(),
        metadata,
    ).map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum DeleteFileResult {
    Success { filename: String },
    Failure { filename: String, reason: String },
}

#[derive(Debug,Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
enum DeleteResponse {
    Busy,
    Success { paths: Vec<DeleteFileResult> },
}

#[actix_web::get("/delete_download/{video_id}")]
pub async fn delete_download(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<AppState>().unwrap().clone();
    let download_state = app.download_cache.entry(video_id.clone()).or_default();
    let mut state = download_state.0.lock().unwrap();
    if state.worker_status.is_busy() {
        return Ok(HttpResponse::Ok().json(DeleteResponse::Busy));
    }
    let db_conn = app.db_pool.get().map_err(ApiError::internal_server)?;
    let entry = select_ytdlp_entry(&db_conn, &video_id).map_err(ApiError::internal_server)?;
    let Some(entry) = entry else { return Ok(HttpResponse::NotFound().finish()); };
    let total_deleted = delete_ytdlp_entry(&db_conn, &video_id).map_err(ApiError::internal_server)?;
    *state = DownloadState::default();
    download_state.1.notify_all();
    drop(state);
    drop(download_state);
    drop(db_conn);
    if total_deleted == 0 { return Ok(HttpResponse::NotFound().finish()); }
    let paths = vec![entry.audio_path, entry.stdout_log_path, entry.stderr_log_path, entry.system_log_path];
    let paths: Vec<String> = paths.into_iter().flatten().collect();
    let paths: Vec<DeleteFileResult> = paths.into_iter().map(|path| {
        match std::fs::remove_file(std::path::PathBuf::from(path.clone())) {
            Ok(()) => DeleteFileResult::Success { filename: path },
            Err(err) => DeleteFileResult::Failure { filename: path, reason: err.to_string() },
        }
    }).collect();
    Ok(HttpResponse::Ok().json(DeleteResponse::Success { paths }))
}

#[actix_web::get("/delete_transcode/{video_id}/{extension}")]
pub async fn delete_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let app = req.app_data::<AppState>().unwrap().clone();
    let transcode_state = app.transcode_cache.entry(transcode_key.clone()).or_default();
    let mut state = transcode_state.0.lock().unwrap();
    if state.worker_status.is_busy() {
        return Ok(HttpResponse::Ok().json(DeleteResponse::Busy));
    }
    let db_conn = app.db_pool.get().map_err(ApiError::internal_server)?;
    let entry = select_ffmpeg_entry(&db_conn, &video_id, audio_ext).map_err(ApiError::internal_server)?;
    let Some(entry) = entry else { return Ok(HttpResponse::NotFound().finish()); };
    let total_deleted = delete_ffmpeg_entry(&db_conn, &video_id, audio_ext).map_err(ApiError::internal_server)?;
    *state = TranscodeState::default();
    transcode_state.1.notify_all();
    drop(state);
    drop(transcode_state);
    drop(db_conn);
    if total_deleted == 0 { return Ok(HttpResponse::NotFound().finish()); }
    let paths = vec![entry.audio_path, entry.stdout_log_path, entry.stderr_log_path, entry.system_log_path];
    let paths: Vec<String> = paths.into_iter().flatten().collect();
    let paths: Vec<DeleteFileResult> = paths.into_iter().map(|path| {
        match std::fs::remove_file(std::path::PathBuf::from(path.clone())) {
            Ok(()) => DeleteFileResult::Success { filename: path },
            Err(err) => DeleteFileResult::Failure { filename: path, reason: err.to_string() },
        }
    }).collect();
    Ok(HttpResponse::Ok().json(DeleteResponse::Success { paths }))
}

#[actix_web::get("/get_downloads")]
pub async fn get_downloads(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let app = req.app_data::<AppState>().unwrap().clone();
    let db_conn = app.db_pool.get().map_err(ApiError::internal_server)?;
    let entries = select_ytdlp_entries(&db_conn).map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(entries))
}

#[actix_web::get("/get_transcodes")]
pub async fn get_transcodes(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let app = req.app_data::<AppState>().unwrap().clone();
    let db_conn = app.db_pool.get().map_err(ApiError::internal_server)?;
    let entries = select_ffmpeg_entries(&db_conn).map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(entries))
}

#[actix_web::get("/get_download/{video_id}")]
pub async fn get_download(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<AppState>().unwrap().clone();
    let db_conn = app.db_pool.get().map_err(ApiError::internal_server)?;
    let entry = select_ytdlp_entry(&db_conn, &video_id).map_err(ApiError::internal_server)?;
    let Some(entry) = entry else {
        return Ok(HttpResponse::NotFound().finish());
    };
    Ok(HttpResponse::Ok().json(entry))
}

#[actix_web::get("/get_transcode/{video_id}/{extension}")]
pub async fn get_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let app = req.app_data::<AppState>().unwrap().clone();
    let db_conn = app.db_pool.get().map_err(ApiError::internal_server)?;
    let entry = select_ffmpeg_entry(&db_conn, &video_id, audio_ext).map_err(ApiError::internal_server)?;
    let Some(entry) = entry else {
        return Ok(HttpResponse::NotFound().finish());
    };
    Ok(HttpResponse::Ok().json(entry))
}

#[actix_web::get("/get_download_state/{video_id}")]
pub async fn get_download_state(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<AppState>().unwrap().clone();
    if let Some(download_state) = app.download_cache.get(&video_id) {
        let download_state = download_state.0.lock().unwrap();
        if download_state.worker_status != WorkerStatus::None {
            return Ok(HttpResponse::Ok().json(download_state.clone()));
        }
    }
    Ok(HttpResponse::NotFound().finish())
}

#[actix_web::get("/get_transcode_state/{video_id}/{extension}")]
pub async fn get_transcode_state(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let app = req.app_data::<AppState>().unwrap().clone();
    if let Some(transcode_state) = app.transcode_cache.get(&transcode_key) {
        let transcode_state = transcode_state.0.lock().unwrap();
        if transcode_state.worker_status != WorkerStatus::None {
            return Ok(HttpResponse::Ok().json(transcode_state.clone()));
        }
    }
    Ok(HttpResponse::NotFound().finish())
}

#[derive(Deserialize)]
struct DownloadLinkParams {
    name: String,
}

#[actix_web::get("/get_download_link/{video_id}/{extension}")]
pub async fn get_download_link(
    req: HttpRequest, path: web::Path<(String, String)>, params: web::Query<DownloadLinkParams>,
) -> actix_web::Result<actix_files::NamedFile> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let app = req.app_data::<AppState>().unwrap().clone();
    let db_conn = app.db_pool.get().map_err(ApiError::internal_server)?;
    let entry = select_ffmpeg_entry(&db_conn, &video_id, audio_ext).map_err(ApiError::internal_server)?;
    let Some(entry) = entry else {
        return Err(error::ErrorNotFound(format!("{0}/{1}", video_id.as_str(), audio_ext.as_str())));
    };
    let Some(audio_path) = entry.audio_path else {
        return Err(error::ErrorNotFound(format!("{0}/{1}", video_id.as_str(), audio_ext.as_str())));
    };
    let audio_path = PathBuf::from(audio_path);
    let file = actix_files::NamedFile::open(audio_path)?;
    // NOTE: You are supposed to use DispositionParam::FilenameExt to specify non-ascii charsets
    //       However I cannot figure out which one to use, and most available sites use nonstandard
    //       filename param to encode utf8 charsets (this is because its only required for
    //       backwards compatibility and most modern browsers dont care about this)
    let attachment = file
        .use_last_modified(true)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(params.name.clone())],
        });
    Ok(attachment)
}

#[actix_web::get("/get_metadata/{video_id}")]
pub async fn get_metadata(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<AppState>().unwrap().clone();
    let metadata = get_metadata_from_cache(video_id, app.metadata_cache).await.map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(metadata.as_ref()))
}

async fn get_metadata_from_cache(video_id: VideoId, cache: MetadataCache) -> Result<Arc<Metadata>, Box<dyn std::error::Error>> {
    if let Some(metadata) = cache.get(&video_id) {
        return Ok(metadata.clone());
    }
    let metadata_url = get_metadata_url(video_id.as_str());
    let response = reqwest::get(metadata_url).await?;
    let metadata = response.text().await?;
    let metadata: Metadata = serde_json::from_str(metadata.as_str())?;
    let metadata = Arc::new(metadata);
    cache.insert(video_id, metadata.clone());
    Ok(metadata)
}
