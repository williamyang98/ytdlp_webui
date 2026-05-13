CREATE TABLE IF NOT EXISTS ffmpeg (
    video_id TEXT NOT NULL,
    audio_ext TEXT NOT NULL,
    status INTEGER DEFAULT 0 NOT NULL,
    unix_time BIGINT NOT NULL,
    stdout_log_path TEXT,
    stderr_log_path TEXT,
    system_log_path TEXT,
    audio_path TEXT,
    PRIMARY KEY (video_id, audio_ext)
)
