// @generated automatically by Diesel CLI.

diesel::table! {
    ffmpeg (video_id, audio_ext) {
        video_id -> Text,
        audio_ext -> Text,
        status -> Integer,
        unix_time -> BigInt,
        stdout_log_path -> Nullable<Text>,
        stderr_log_path -> Nullable<Text>,
        system_log_path -> Nullable<Text>,
        audio_path -> Nullable<Text>,
    }
}

diesel::table! {
    ytdlp (video_id) {
        video_id -> Text,
        status -> Integer,
        unix_time -> BigInt,
        stdout_log_path -> Nullable<Text>,
        stderr_log_path -> Nullable<Text>,
        system_log_path -> Nullable<Text>,
        audio_path -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(ffmpeg, ytdlp,);
