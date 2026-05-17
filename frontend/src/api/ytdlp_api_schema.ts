import * as z from "zod";

// https://zod.dev/api
export const WorkerStatusSchema = z.enum(["queued", "running", "finished", "failed"]);
export const AudioExtensionSchema = z.enum(["m4a", "aac", "mp3", "webm"]);

function convert_unix_time(seconds: number): Date {
  return new Date(seconds * 1000);
}

function convert_video_id(id: string): VideoId {
  return id;
}

export const DownloadStateSchema = z.object({
  worker_status: WorkerStatusSchema,
  file_cached: z.boolean(),
  fail_reason: z.string().optional(),
  start_time_unix: z.int().transform(convert_unix_time),
  end_time_unix: z.int().transform(convert_unix_time),
  eta_seconds: z.int().optional(),
  elapsed_seconds: z.int().optional(),
  downloaded_bytes: z.int().optional(),
  total_bytes: z.int().optional(),
  speed_bytes: z.int().optional(),
});

export const TranscodeStateSchema = z.object({
  worker_status: WorkerStatusSchema,
  file_cached: z.boolean(),
  fail_reason: z.string().optional(),
  start_time_unix: z.int().transform(convert_unix_time),
  end_time_unix: z.int().transform(convert_unix_time),
  source_duration_milliseconds: z.int().optional(),
  source_start_time_milliseconds: z.int().optional(),
  source_speed_bits: z.int().optional(),
  transcode_duration_milliseconds: z.int().optional(),
  transcode_size_bytes: z.int().optional(),
  transcode_speed_bits: z.int().optional(),
  transcode_speed_factor: z.float32().optional(),
});

export const YtdlpRowSchema = z.object({
  video_id: z.string().transform(convert_video_id),
  status: WorkerStatusSchema,
  unix_time: z.int().transform(convert_unix_time),
  stdout_log_path: z.string().optional(),
  stderr_log_path: z.string().optional(),
  system_log_path: z.string().optional(),
  audio_path: z.string().optional(),
});

export const FfmpegRowSchema = z.object({
  video_id: z.string().transform(convert_video_id),
  audio_ext: AudioExtensionSchema,
  status: WorkerStatusSchema,
  unix_time: z.int().transform(convert_unix_time),
  stdout_log_path: z.string().optional(),
  stderr_log_path: z.string().optional(),
  system_log_path: z.string().optional(),
  audio_path: z.string().optional(),
});

export const DeleteFileResultSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("success"), filename: z.string() }),
  z.object({ type: z.literal("failure"), filename: z.string(), reason: z.string() }),
]);

export const DeleteResponseSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("busy") }),
  z.object({ type: z.literal("success"), paths: DeleteFileResultSchema.array() }),
]);

export const RequestTranscodeResponseSchema = z.object({
  download_status: WorkerStatusSchema,
  transcode_status: WorkerStatusSchema,
  is_skip_transcode: z.boolean(),
});

export type WorkerStatus = z.infer<typeof WorkerStatusSchema>;
export type AudioExtension = z.infer<typeof AudioExtensionSchema>;
export type DownloadState = z.infer<typeof DownloadStateSchema>;
export type TranscodeState = z.infer<typeof TranscodeStateSchema>;
export type YtdlpRow = z.infer<typeof YtdlpRowSchema>;
export type FfmpegRow = z.infer<typeof FfmpegRowSchema>;
export type DeleteFileResult = z.infer<typeof DeleteFileResultSchema>;
export type DeleteResponse = z.infer<typeof DeleteResponseSchema>;
export type RequestTranscodeResponse = z.infer<typeof RequestTranscodeResponseSchema>;
export type VideoId = string;
export interface TranscodeKey {
  video_id: VideoId;
  audio_ext: AudioExtension;
}
