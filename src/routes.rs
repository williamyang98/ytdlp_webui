use actix_web::{web, HttpRequest, Responder};
use crate::database::{
    DatabasePool, VideoId, AudioExtension,
};
use crate::worker_download::{try_start_download_worker, DownloadCache, DOWNLOAD_AUDIO_EXT};
use crate::worker_transcode::{try_start_transcode_worker, TranscodeCache, TranscodeKey};
use crate::app::{AppConfig, WorkerThreadPool};

#[actix_web::get("/index.html")]
pub async fn index() -> impl Responder {
    "Hello World!"
}

#[actix_web::post("/audio/{video_id}/{extension}")]
pub async fn request_audio(req: HttpRequest, path: web::Path<(String, String)>) -> actix_web::Result<impl Responder> {
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
    let download_worker_status = try_start_download_worker(
        video_id.clone(),
        download_cache.clone(), app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
    );
    log::debug!("Download worker: status={download_worker_status:?}, id={0}.{1}", video_id.as_str(), DOWNLOAD_AUDIO_EXT.as_str());
    // skip transcode
    if audio_ext == DOWNLOAD_AUDIO_EXT {
        log::debug!("Audio file is in downloaded format already: {0}", audio_ext.as_str()); 
        return Ok("Skipping transcode since already in downloaded format".to_string());
    }
    // transcode
    let transcode_worker_status = try_start_transcode_worker(
        transcode_key.clone(),
        download_cache, transcode_cache, app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
    );
    log::debug!("Transcode worker: status={transcode_worker_status:?}, id={0}.{1}", video_id.as_str(), audio_ext.as_str());
    return Ok(format!("Queueing download and transcode threads for: id={0}.{1}", video_id.as_str(), audio_ext.as_str()));
}

