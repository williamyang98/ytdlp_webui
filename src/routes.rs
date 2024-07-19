use actix_web::{error, web, HttpRequest, HttpResponse, Responder};
// use serde::Serialize;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use derive_more::{Display, Error};
use crate::database::{
    DatabasePool, VideoId, VideoIdError, AudioExtension, WorkerStatus, WorkerTable,
    select_worker_fields, delete_worker_entry,
    select_ytdlp_entries, select_ffmpeg_entries,
};
use crate::worker_download::{try_start_download_worker, DownloadCache, DOWNLOAD_AUDIO_EXT};
use crate::worker_transcode::{try_start_transcode_worker, TranscodeCache, TranscodeKey};
use crate::app::{AppConfig, WorkerThreadPool};

// #[derive(Debug, Display, Error)]
// enum CustomError {
//     #[display(fmt="Bad video id provided")]
//     BadVideoId { id: String, reason: VideoIdError }, 
//     #[display(fmt="Invalid audio extension provided")]
//     InvalidAudioExtension { ext: String },
//     #[display(fmt="Unsupported audio extension provided")]
//     UnsupportedAudioExtension { ext: AudioExtension }, 
// }

#[actix_web::get("/request_transcode/{video_id}/{extension}")]
pub async fn request_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<impl Responder> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = match VideoId::try_new(video_id.as_str()) {
        Ok(video_id) => video_id,
        Err(err) => return Ok(format!("Invalid video id: {err:?}")),
    };
    let audio_ext = match AudioExtension::try_from(audio_ext.as_str()) {
        Ok(ext) => ext,
        Err(err) => return Ok(format!("Invalid audio extension format: {err:?}")),
    };
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let download_cache = req.app_data::<DownloadCache>().unwrap().clone();
    let transcode_cache = req.app_data::<TranscodeCache>().unwrap().clone();
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    let worker_thread_pool: WorkerThreadPool = req.app_data::<WorkerThreadPool>().unwrap().clone();
    let app_config = req.app_data::<AppConfig>().unwrap().clone();
    // download audio file
    log::debug!("Try start download worker: id={0}", transcode_key.as_str());
    let download_worker_status = try_start_download_worker(
        video_id.clone(),
        download_cache.clone(), app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
    );
    log::debug!("Download worker: status={download_worker_status:?}, id={0}", transcode_key.as_str());
    // skip transcode
    if audio_ext == DOWNLOAD_AUDIO_EXT {
        log::debug!("Audio file is in downloaded format already: {0}", audio_ext.as_str()); 
        return Ok("Skipping transcode since already in downloaded format".to_string());
    }
    // transcode
    log::debug!("Try start transcode worker: id={0}", transcode_key.as_str());
    let transcode_worker_status = try_start_transcode_worker(
        transcode_key.clone(),
        download_cache, transcode_cache, app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
    );
    log::debug!("Transcode worker: status={transcode_worker_status:?}, id={0}.{1}", video_id.as_str(), audio_ext.as_str());
    Ok(format!("Queueing download and transcode threads for: id={0}.{1}", video_id.as_str(), audio_ext.as_str()))
}

#[actix_web::get("/delete_transcode/{video_id}/{extension}")]
pub async fn delete_transcode(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<impl Responder> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = match VideoId::try_new(video_id.as_str()) {
        Ok(video_id) => video_id,
        Err(err) => return Ok(format!("Invalid video id: {err:?}")),
    };
    let audio_ext = match AudioExtension::try_from(audio_ext.as_str()) {
        Ok(ext) => ext,
        Err(err) => return Ok(format!("Invalid audio extension format: {err:?}")),
    };
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    if audio_ext == DOWNLOAD_AUDIO_EXT {
        let download_cache = req.app_data::<DownloadCache>().unwrap().clone();
        let download_state = download_cache.entry(video_id.clone()).or_default();
        let mut state = download_state.0.lock().unwrap();
        if state.worker_status.is_busy() {
            return Ok("Download is in progress, cannot delete".to_owned()); 
        }
        let db_conn = db_pool.get().map_err(error::ErrorInternalServerError)?;
        let total_deleted = delete_worker_entry(&db_conn, &video_id, audio_ext, WorkerTable::YTDLP)
            .map_err(error::ErrorInternalServerError)?;
        state.worker_status = WorkerStatus::None;
        download_state.1.notify_all();
        log::info!("Deleted download: id={0}.{1}, total={2}", video_id.as_str(), audio_ext.as_str(), total_deleted);
        Ok(format!("Deleted download: id={0}.{1}, total={2}", video_id.as_str(), audio_ext.as_str(), total_deleted))
    } else {
        let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
        let transcode_cache = req.app_data::<TranscodeCache>().unwrap().clone();
        let transcode_state = transcode_cache.entry(transcode_key.clone()).or_default();
        let mut state = transcode_state.0.lock().unwrap();
        if state.worker_status.is_busy() {
            return Ok("Transcode in progress, cannot delete".to_owned());
        }
        let db_conn = db_pool.get().map_err(error::ErrorInternalServerError)?;
        let total_deleted = delete_worker_entry(&db_conn, &video_id, audio_ext, WorkerTable::FFMPEG)
            .map_err(error::ErrorInternalServerError)?;
        state.worker_status = WorkerStatus::None;
        transcode_state.1.notify_all();
        log::info!("Deleted transcode: id={0}.{1}, total={2}", video_id.as_str(), audio_ext.as_str(), total_deleted);
        Ok(format!("Deleted transcode: id={0}.{1}, total={2}", video_id.as_str(), audio_ext.as_str(), total_deleted))
    }
}

#[actix_web::get("/get_downloads")]
pub async fn get_downloads(req: HttpRequest) -> actix_web::Result<impl Responder> {
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    let db_conn = db_pool.get().map_err(error::ErrorInternalServerError)?;
    let entries = select_ytdlp_entries(&db_conn).map_err(error::ErrorInternalServerError)?;
    Ok(web::Json(entries))
}

#[actix_web::get("/get_transcodes")]
pub async fn get_transcodes(req: HttpRequest) -> actix_web::Result<impl Responder> {
    let db_pool = req.app_data::<DatabasePool>().unwrap().clone();
    let db_conn = db_pool.get().map_err(error::ErrorInternalServerError)?;
    let entries = select_ffmpeg_entries(&db_conn).map_err(error::ErrorInternalServerError)?;
    Ok(web::Json(entries))
}

#[actix_web::get("/get_download_state/{video_id}/{extension}")]
pub async fn get_download_state(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<impl Responder> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = match VideoId::try_new(video_id.as_str()) {
        Ok(video_id) => video_id,
        Err(err) => return Ok(format!("Invalid video id: {err:?}")),
    };
    let audio_ext = match AudioExtension::try_from(audio_ext.as_str()) {
        Ok(ext) => ext,
        Err(err) => return Ok(format!("Invalid audio extension format: {err:?}")),
    };
    if audio_ext != DOWNLOAD_AUDIO_EXT {
        return Ok(format!("Download can only be in {0} but got {1}", DOWNLOAD_AUDIO_EXT.as_str(), audio_ext.as_str()));
    }
    let download_cache = req.app_data::<DownloadCache>().unwrap().clone();
    if let Some(download_state) = download_cache.get(&video_id) {
        return Ok(serde_json::to_string(&*download_state.0.lock().unwrap())?);
    }
    Ok("No download state".to_owned())
}

#[actix_web::get("/get_transcode_state/{video_id}/{extension}")]
pub async fn get_transcode_state(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<impl Responder> {
    let (video_id, audio_ext) = path.into_inner();
    let video_id = match VideoId::try_new(video_id.as_str()) {
        Ok(video_id) => video_id,
        Err(err) => return Ok(format!("Invalid video id: {err:?}")),
    };
    let audio_ext = match AudioExtension::try_from(audio_ext.as_str()) {
        Ok(ext) => ext,
        Err(err) => return Ok(format!("Invalid audio extension format: {err:?}")),
    };
    if audio_ext == DOWNLOAD_AUDIO_EXT {
        return Ok(format!("Transcode cannot be in {0}", audio_ext.as_str()));
    }
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let transcode_cache = req.app_data::<TranscodeCache>().unwrap().clone();
    if let Some(transcode_state) = transcode_cache.get(&transcode_key) {
        return Ok(serde_json::to_string(&*transcode_state.0.lock().unwrap())?);
    }
    Ok("No transcode state".to_owned())
}
