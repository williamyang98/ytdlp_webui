use std::path::PathBuf;

use actix_web::error::ErrorInternalServerError;
use actix_web::{web, HttpRequest, Responder};
use crate::database::{
    DatabasePool, VideoId, AudioExtension, WorkerTable,
    select_worker_fields,
};
use crate::worker_download::{try_start_download_worker, DownloadCache, DownloadStartStatus, DOWNLOAD_AUDIO_EXT};
use crate::worker_transcode::{try_start_transcode_worker, TranscodeCache, TranscodeKey, TranscodeStartStatus};
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
    // TODO: Make it so that we can have requests buffered in a pipeline
    //       Multiple transcodes waiting for multiple downloads
    // download audio file
    let download_cache = req.app_data::<DownloadCache>().unwrap().clone();
    let db_pool: DatabasePool = req.app_data::<DatabasePool>().unwrap().clone();
    let worker_thread_pool: WorkerThreadPool = req.app_data::<WorkerThreadPool>().unwrap().clone();
    let app_config = req.app_data::<AppConfig>().unwrap().clone();
    let force_redownload = false;
    let download_res = try_start_download_worker(
        video_id.clone(), force_redownload,
        download_cache, app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
    );
    match download_res {
        Ok(DownloadStartStatus::Finished) => {},
        Ok(status) => return Ok(format!("Download status: id={0}, state={1:?}", video_id.as_str(), status)),
        Err(err) => return Ok(format!("Download start failed: {err:?}")),
    };
    // skip transcode step if same format as download
    let audio_path = {
        let db_conn = db_pool.get().map_err(ErrorInternalServerError)?;
        let audio_path: Option<String> = select_worker_fields(
            &db_conn, &video_id, DOWNLOAD_AUDIO_EXT, WorkerTable::YTDLP,
            &["audio_path"], |row| row.get(0),
        ).map_err(ErrorInternalServerError)?;
        let Some(audio_path) = audio_path else {
            return Ok("Missing downloaded audio path".to_owned());
        };
        audio_path
    };
    let audio_path = PathBuf::from(audio_path);
    // TODO: If this fails then we were intentionally messing up the index, how do we recover from this?
    if !audio_path.exists() {
        return Ok(format!("Audio path is missing: {0}", audio_path.to_str().unwrap()));
    }
    if audio_ext == DOWNLOAD_AUDIO_EXT {
        return Ok(format!("Audio file is in downloaded format already: {0}", audio_path.to_str().unwrap())); 
    }
    // transcode
    let transcode_cache = req.app_data::<TranscodeCache>().unwrap().clone();
    let force_retranscode = false;
    let transcode_key = TranscodeKey { video_id: video_id.clone(), audio_ext };
    let transcode_res = try_start_transcode_worker(
        audio_path.clone(), transcode_key.clone(), force_retranscode, 
        transcode_cache, app_config.clone(), db_pool.clone(), worker_thread_pool.clone(),
    );
    match transcode_res {
        Ok(TranscodeStartStatus::Finished) => {
            let db_conn = db_pool.get().map_err(ErrorInternalServerError)?;
            let audio_path: Option<String> = select_worker_fields(
                &db_conn, &video_id, audio_ext, WorkerTable::FFMPEG,
                &["audio_path"], |row| row.get(0),
            ).map_err(ErrorInternalServerError)?;
            let Some(audio_path) = audio_path else {
                return Ok("Missing transcoded audio path".to_owned());
            };
            let audio_path = PathBuf::from(audio_path);
            if !audio_path.exists() {
                return Ok(format!("Audio path is missing: {0}", audio_path.to_str().unwrap()));
            }
            return Ok(format!("Audio file is transcoded: {0}", audio_path.to_str().unwrap())); 
        },
        Ok(status) => return Ok(format!("Transcode status: id={0}, state={1:?}", transcode_key.as_str(), status)),
        Err(err) => return Ok(format!("Transcode start failed: {err:?}")),
    };
}

