use std::{path::PathBuf, sync::Arc};
use actix_web::{middleware, web, App as ActixApp, HttpServer};
use clap::Parser;
use app::app_config::AppConfig;
use app::app::App;
use server::routes;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Url of server
    #[arg(long, default_value = "0.0.0.0")]
    url: String,
    /// Port of server
    #[arg(long, default_value_t = 8080)]
    port: u16,
    /// Maximum number of transcode threads
    #[arg(long, default_value_t = 0)]
    total_transcode_threads: usize,
    /// Maximum number of worker threads
    #[arg(long, default_value_t = 0)]
    total_worker_threads: usize,
    /// Data folder
    #[arg(long, default_value = "./data", value_parser = validate_is_directory_empty_or_exists)]
    data_folder: PathBuf,
    /// Static website folder
    #[arg(long, default_value = "./static", value_parser = validate_is_directory_empty_or_exists)]
    static_folder: PathBuf,
}

fn validate_is_directory_empty_or_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && !path.is_dir() {
        Err("Cannot write to existing path that is not a directory".into())
    } else {
        Ok(path)
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "INFO");
    }
    env_logger::init();

    let total_transcode_threads: usize = match args.total_transcode_threads {
        0 => std::thread::available_parallelism().map(|v| v.get()).unwrap_or(1),
        x => x,
    };
    let total_worker_threads: usize = match args.total_worker_threads {
        0 => std::thread::available_parallelism().map(|v| v.get()).unwrap_or(1),
        x => x,
    };
    let mut app_config = AppConfig::new(&args.data_folder, &args.static_folder)?;
    app_config.total_transcode_threads = total_transcode_threads;
    let app = App::new(app_config.clone())?;
    let app = Arc::new(app);
    // start server
    const API_PREFIX: &str = "/api/v1";
    HttpServer::new(move || {
        ActixApp::new()
            .app_data(app.clone())
            .service(web::scope(API_PREFIX)
                .service(routes::request_transcode)
                .service(routes::delete_transcode)
                .service(routes::delete_download)
                .service(routes::get_downloads)
                .service(routes::get_transcodes)
                .service(routes::get_download)
                .service(routes::get_transcode)
                .service(routes::get_download_state)
                .service(routes::get_transcode_state)
                .service(routes::get_download_link)
                .service(routes::get_metadata)
            )
            .service(actix_files::Files::new("/data", &app_config.data_folder).show_files_listing())
            .service(actix_files::Files::new("/", &app_config.static_folder).index_file("index.html"))
            // NOTE: There is little benefit to using compress middleware when serving audio files
            // since they are already extremely compressed. Additionally it also ends up removing
            // the Content-Length header from the downloads since the file is being streamed.
            // This has the effect of removing any progress bar on the download which is a bad experience.
            // .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
    })
    .bind((args.url, args.port))?
    .workers(total_worker_threads)
    .run()
    .await?;
    Ok(())
}
