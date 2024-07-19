use actix_web::{
    error, web, HttpRequest, HttpResponse, 
    http::{StatusCode, header::ContentType},
};
use serde::Serialize;
use derive_more::Display;
use crate::database::{
    DatabasePool, VideoId, VideoIdError, AudioExtension, WorkerStatus, WorkerTable,
    delete_worker_entry,
    select_ytdlp_entries, select_ffmpeg_entries,
};
use crate::worker_download::{try_start_download_worker, DownloadCache, DOWNLOAD_AUDIO_EXT};
use crate::worker_transcode::{try_start_transcode_worker, TranscodeCache, TranscodeKey};
use crate::app::{AppConfig, WorkerThreadPool};

#[derive(Debug,Clone,Serialize,Display)]
#[display(fmt = "UserApiError({},{})", error, status_code)]
struct ApiError {
    error: String,
    #[serde(skip)]
    status_code: StatusCode,
}

impl ApiError {
    fn new(error: String, status_code: StatusCode) -> Self {
        Self { error, status_code }
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

fn error_invalid_video_id(id: String, err: VideoIdError) -> ApiError {
    ApiError {
        error: format!("invalid video id {id}: {err:?}"),
        status_code: StatusCode::BAD_REQUEST,
    }
}

fn error_invalid_audio_extension(ext: String) -> ApiError {
    ApiError {
        error: format!("invalid audio extension: {ext}"),
        status_code: StatusCode::BAD_REQUEST,
    }
}

fn error_internal_server(err: impl std::fmt::Debug) -> ApiError {
    ApiError {
        error: format!("internal server error: {err:?}"),
        status_code: StatusCode::INTERNAL_SERVER_ERROR,
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
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| error_invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| error_invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let download_cache = req.app_data::<DownloadCache>().unwrap().clone();
    let transcode_cache = req.app_data::<TranscodeCache>().unwrap().clone();
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    let worker_thread_pool: WorkerThreadPool = req.app_data::<WorkerThreadPool>().unwrap().clone();
    let app_config = req.app_data::<AppConfig>().unwrap().clone();
    // download audio file
    let mut response = RequestTranscodeResponse::default();
    response.download_status = try_start_download_worker(
        video_id.clone(),
        download_cache.clone(), app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
    ).map_err(error_internal_server)?;
    if audio_ext == DOWNLOAD_AUDIO_EXT {
        // skip transcode
        response.is_skip_transcode = true;
    } else {
        // transcode
        response.transcode_status = try_start_transcode_worker(
            transcode_key.clone(),
            download_cache, transcode_cache, app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
        ).map_err(error_internal_server)?;
    }
    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug,Default,Serialize)]
enum DeleteTranscodeResponse {
    #[default]
    None,
    Busy,
    Success(usize),
}

#[actix_web::get("/delete_transcode/{video_id}/{extension}")]
pub async fn delete_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| error_invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| error_invalid_audio_extension(audio_ext))?;
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    let total_deleted = if audio_ext == DOWNLOAD_AUDIO_EXT {
        let download_cache = req.app_data::<DownloadCache>().unwrap().clone();
        let download_state = download_cache.entry(video_id.clone()).or_default();
        let mut state = download_state.0.lock().unwrap();
        if state.worker_status.is_busy() {
            return Ok(HttpResponse::Ok().json(DeleteTranscodeResponse::Busy)); 
        }
        let db_conn = db_pool.get().map_err(error::ErrorInternalServerError)?;
        let total_deleted = delete_worker_entry(&db_conn, &video_id, audio_ext, WorkerTable::YTDLP)
            .map_err(error::ErrorInternalServerError)?;
        state.worker_status = WorkerStatus::None;
        download_state.1.notify_all();
        total_deleted
    } else {
        let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
        let transcode_cache = req.app_data::<TranscodeCache>().unwrap().clone();
        let transcode_state = transcode_cache.entry(transcode_key.clone()).or_default();
        let mut state = transcode_state.0.lock().unwrap();
        if state.worker_status.is_busy() {
            return Ok(HttpResponse::Ok().json(DeleteTranscodeResponse::Busy));
        }
        let db_conn = db_pool.get().map_err(error::ErrorInternalServerError)?;
        let total_deleted = delete_worker_entry(&db_conn, &video_id, audio_ext, WorkerTable::FFMPEG)
            .map_err(error::ErrorInternalServerError)?;
        state.worker_status = WorkerStatus::None;
        transcode_state.1.notify_all();
        total_deleted
    };
    if total_deleted > 0 {
        Ok(HttpResponse::Ok().json(DeleteTranscodeResponse::Success(total_deleted)))
    } else {
        Ok(HttpResponse::Ok().json(DeleteTranscodeResponse::None))
    }
}

#[actix_web::get("/get_downloads")]
pub async fn get_downloads(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    let db_conn = db_pool.get().map_err(error_internal_server)?;
    let entries = select_ytdlp_entries(&db_conn).map_err(error_internal_server)?;
    Ok(HttpResponse::Ok().json(entries))
}

#[actix_web::get("/get_transcodes")]
pub async fn get_transcodes(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    let db_conn = db_pool.get().map_err(error_internal_server)?;
    let entries = select_ffmpeg_entries(&db_conn).map_err(error_internal_server)?;
    Ok(HttpResponse::Ok().json(entries))
}

#[actix_web::get("/get_download_state/{video_id}/{extension}")]
pub async fn get_download_state(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| error_invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| error_invalid_audio_extension(audio_ext))?;
    if audio_ext != DOWNLOAD_AUDIO_EXT {
        return Err(ApiError::new(
            format!("Downloads can only have the '{0}' extension, but got '{1}'", DOWNLOAD_AUDIO_EXT.as_str(), audio_ext.as_str()),
            StatusCode::BAD_REQUEST,
        ).into());
    }
    let download_cache = req.app_data::<DownloadCache>().unwrap().clone();
    if let Some(download_state) = download_cache.get(&video_id) {
        return Ok(HttpResponse::Ok().json(*download_state.0.lock().unwrap()));
    }
    Ok(HttpResponse::NotFound().finish())
}

#[actix_web::get("/get_transcode_state/{video_id}/{extension}")]
pub async fn get_transcode_state(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = VideoId::try_new(video_id.as_str()).map_err(|e| error_invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| error_invalid_audio_extension(audio_ext))?;
    if audio_ext == DOWNLOAD_AUDIO_EXT {
        return Err(ApiError::new(
            format!("Transcodes cannot have the '{0}' extension", DOWNLOAD_AUDIO_EXT.as_str()),
            StatusCode::BAD_REQUEST,
        ).into());
    }
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let transcode_cache = req.app_data::<TranscodeCache>().unwrap().clone();
    if let Some(transcode_state) = transcode_cache.get(&transcode_key) {
        return Ok(HttpResponse::Ok().json(*transcode_state.0.lock().unwrap()));
    }
    Ok(HttpResponse::NotFound().finish())
}
