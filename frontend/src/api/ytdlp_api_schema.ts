import * as z from "zod";
import { type VideoId } from "./youtube_api_schema.ts";

// https://zod.dev/api
export const WorkerStatusSchema = z.enum(["queued", "running", "finished", "failed"]);
export const AudioExtensionSchema = z.enum(["m4a", "aac", "mp3", "webm"]);

function convert_unix_time(seconds: number): Date {
  return new Date(seconds * 1000);
}

function convert_video_id(id: string): VideoId {
  return id;
}

function into_option<T>(value: T | undefined | null): T | undefined {
  if (value === null || value === undefined) return undefined;
  return value;
}

export const DownloadStateSchema = z.object({
  id: z.uuidv4(),
  worker_status: WorkerStatusSchema,
  file_cached: z.boolean(),
  fail_reason: z.string().nullish().transform(into_option),
  start_time_unix: z.int().transform(convert_unix_time),
  end_time_unix: z.int().transform(convert_unix_time),
  eta_seconds: z.int().nullish().transform(into_option),
  elapsed_seconds: z.int().nullish().transform(into_option),
  downloaded_bytes: z.int().nullish().transform(into_option),
  total_bytes: z.int().nullish().transform(into_option),
  speed_bytes: z.int().nullish().transform(into_option),
});

export const TranscodeStateSchema = z.object({
  id: z.uuidv4(),
  worker_status: WorkerStatusSchema,
  file_cached: z.boolean(),
  fail_reason: z.string().nullish().transform(into_option),
  start_time_unix: z.int().transform(convert_unix_time),
  end_time_unix: z.int().transform(convert_unix_time),
  source_duration_milliseconds: z.int().nullish().transform(into_option),
  source_start_time_milliseconds: z.int().nullish().transform(into_option),
  source_speed_bits: z.int().nullish().transform(into_option),
  transcode_duration_milliseconds: z.int().nullish().transform(into_option),
  transcode_size_bytes: z.int().nullish().transform(into_option),
  transcode_speed_bits: z.int().nullish().transform(into_option),
  transcode_speed_factor: z.float32().nullish().transform(into_option),
});

export const YtdlpRowSchema = z.object({
  video_id: z.string().transform(convert_video_id),
  status: WorkerStatusSchema,
  unix_time: z.int().transform(convert_unix_time),
  stdout_log_path: z.string().nullish().transform(into_option),
  stderr_log_path: z.string().nullish().transform(into_option),
  system_log_path: z.string().nullish().transform(into_option),
  audio_path: z.string().nullish().transform(into_option),
});

export const FfmpegRowSchema = z.object({
  video_id: z.string().transform(convert_video_id),
  audio_ext: AudioExtensionSchema,
  status: WorkerStatusSchema,
  unix_time: z.int().transform(convert_unix_time),
  stdout_log_path: z.string().nullish().transform(into_option),
  stderr_log_path: z.string().nullish().transform(into_option),
  system_log_path: z.string().nullish().transform(into_option),
  audio_path: z.string().nullish().transform(into_option),
});

export const DeleteFileResultSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("success"), filename: z.string() }),
  z.object({ type: z.literal("failure"), filename: z.string(), reason: z.string() }),
]);

export const DeleteResponseSchema = DeleteFileResultSchema.array();

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
export type DownloadKey = VideoId;
export interface TranscodeKey {
  video_id: VideoId;
  audio_ext: AudioExtension;
}

export function is_worker_running(status: WorkerStatus) {
  switch (status) {
    case "queued": return true;
    case "running": return true;
    case "failed": return false;
    case "finished": return false;
  }
}
