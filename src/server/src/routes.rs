use actix_web::http::StatusCode;
use actix_web::http::header::{ContentDisposition, ContentType, DispositionParam, DispositionType}; 
use actix_web::{error, web, HttpRequest, HttpResponse};
use anyhow::Context;
use app::app::{App, DeleteResponse};
use app::database::{AudioExtension, WorkerStatus, TranscodeKey};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use youtube_api::{PlaylistId, PlaylistIdError, VideoId, VideoIdError};

#[derive(Debug,Clone,Serialize,Display)]
#[display("UserApiError({},{})", error, status_code)]
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

    fn invalid_playlist_id(id: String, err: PlaylistIdError) -> Self {
        Self {
            error: format!("invalid playlist id {id}: {err:?}"),
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
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let app = req.app_data::<Arc<App>>().unwrap();
    // download audio file
    let mut response = RequestTranscodeResponse::default();
    response.download_status = app.start_download(&video_id)
        .context("Failed to start download worker")
        .map_err(ApiError::internal_server)?
        .get_status();
    // download video info
    let video_info = app.get_youtube_video(&video_id).await;
    let video_info = match video_info {
        Ok(video_info) => Some(video_info),
        Err(err) => {
            log::warn!("Failed to retrieve youtube api video info for video_id={0} so skipping embedded thumbnail for audio_ext={1}: {2}", &video_id, audio_ext.as_str(), err);
            None
        },
    };
    // transcode
    response.transcode_status = app.start_transcode(&transcode_key, video_info)
        .context("Failed to start transcode worker")
        .map_err(ApiError::internal_server)?
        .get_status();
    Ok(HttpResponse::Ok().json(response))
}

#[actix_web::get("/delete_download/{video_id}")]
pub async fn delete_download(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<Arc<App>>().unwrap().clone();
    let result = web::block(move || app.delete_download(&video_id))
        .await
        .map_err(ApiError::internal_server)?
        .map_err(ApiError::internal_server)?;
    let Some(result) = result else {
        return Ok(HttpResponse::NotFound().finish());
    };
    match result {
        DeleteResponse::Busy => Ok(HttpResponse::Conflict().body("Download worker is busy")), 
        DeleteResponse::Success { paths } => Ok(HttpResponse::Ok().json(paths)),
    }
}

#[actix_web::get("/delete_transcode/{video_id}/{extension}")]
pub async fn delete_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let app = req.app_data::<Arc<App>>().unwrap().clone();
    let result = web::block(move || app.delete_transcode(&transcode_key))
        .await
        .map_err(ApiError::internal_server)?
        .map_err(ApiError::internal_server)?;
    let Some(result) = result else {
        return Ok(HttpResponse::NotFound().finish());
    };
    match result {
        DeleteResponse::Busy => Ok(HttpResponse::Conflict().body("Transcode worker is busy")), 
        DeleteResponse::Success { paths } => Ok(HttpResponse::Ok().json(paths)),
    }
}

#[actix_web::get("/get_downloads")]
pub async fn get_downloads(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let app = req.app_data::<Arc<App>>().unwrap();
    let entries = app.get_downloads().map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(entries))
}

#[actix_web::get("/get_transcodes")]
pub async fn get_transcodes(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let app = req.app_data::<Arc<App>>().unwrap();
    let entries = app.get_transcodes().map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(entries))
}

#[actix_web::get("/get_download/{video_id}")]
pub async fn get_download(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<Arc<App>>().unwrap();
    let entry = app.get_download(&video_id).map_err(ApiError::internal_server)?;
    let Some(entry) = entry else {
        return Ok(HttpResponse::NotFound().finish());
    };
    Ok(HttpResponse::Ok().json(entry))
}

#[actix_web::get("/get_transcode/{video_id}/{extension}")]
pub async fn get_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id, audio_ext };
    let app = req.app_data::<Arc<App>>().unwrap();
    let entry = app.get_transcode(&transcode_key).map_err(ApiError::internal_server)?;
    let Some(entry) = entry else {
        return Ok(HttpResponse::NotFound().finish());
    };
    Ok(HttpResponse::Ok().json(entry))
}

#[actix_web::get("/get_download_state/{video_id}")]
pub async fn get_download_state(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<Arc<App>>().unwrap();
    let worker = app.get_download_worker(&video_id);
    let Some(worker) = worker else {
        return Ok(HttpResponse::NotFound().finish());
    };
    Ok(HttpResponse::Ok().json(worker.get_state()))
}

#[actix_web::get("/get_transcode_state/{video_id}/{extension}")]
pub async fn get_transcode_state(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<HttpResponse> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let app = req.app_data::<Arc<App>>().unwrap();
    let worker = app.get_transcode_worker(&transcode_key);
    let Some(worker) = worker else {
        return Ok(HttpResponse::NotFound().finish());
    };
    Ok(HttpResponse::Ok().json(worker.get_state()))
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
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let audio_ext = AudioExtension::try_from(audio_ext.as_str()).map_err(|_| ApiError::invalid_audio_extension(audio_ext))?;
    let transcode_key = TranscodeKey { video_id, audio_ext };
    let app = req.app_data::<Arc<App>>().unwrap();
    let path = app.get_download_abspath(&transcode_key).map_err(ApiError::internal_server)?;
    let Some(path) = path else {
        return Err(error::ErrorNotFound(transcode_key.as_str()));
    };
    let file = actix_files::NamedFile::open(path)?;
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

#[actix_web::get("/youtube_api/video/{video_id}")]
pub async fn get_youtube_video(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let video_id = path.into_inner();
    let video_id: VideoId = video_id.as_str().try_into().map_err(|e| ApiError::invalid_video_id(video_id, e))?;
    let app = req.app_data::<Arc<App>>().unwrap().clone();
    let video_info = app.get_youtube_video(&video_id).await.map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(video_info.as_ref()))
}

#[actix_web::get("/youtube_api/playlist/{playlist_id}")]
pub async fn get_youtube_playlist(req: HttpRequest, path: web::Path<String>) -> actix_web::Result<HttpResponse> {
    let playlist_id = path.into_inner();
    let playlist_id: PlaylistId = playlist_id.as_str().try_into().map_err(|e| ApiError::invalid_playlist_id(playlist_id, e))?;
    let app = req.app_data::<Arc<App>>().unwrap().clone();
    let playlist_info = app.get_youtube_playlist(&playlist_id).await.map_err(ApiError::internal_server)?;
    Ok(HttpResponse::Ok().json(playlist_info.as_ref()))
}

