import * as z from "zod";

const BASE_URL =
  import.meta.env?.DEV ?
  "http://localhost:8080" :
  `${window.location.protocol}//${window.location.host}`;
const API_URL = "api/v1";

const WorkerStatusSchema = z.enum(["queued", "running", "finished", "failed"]);
const AudioExtensionSchema = z.enum(["m4a", "aac", "mp3", "webm"]);

const TranscodeStateSchema = z.object({
  worker_status: WorkerStatusSchema,
  file_cached: z.boolean(),
  fail_reason: z.string().optional(),
  start_time_unix: z.int(),
  end_time_unix: z.int(),
  source_duration_milliseconds: z.int().optional(),
  source_start_time_milliseconds: z.int().optional(),
  source_speed_bits: z.int().optional(),
  transcode_duration_milliseconds: z.int().optional(),
  transcode_size_bytes: z.int().optional(),
  transcode_speed_bits: z.int().optional(),
  transcode_speed_factor: z.float32().optional(),
});

const FfmpegRowSchema = z.object({
  video_id: z.string(),
  audio_ext: AudioExtensionSchema,
  status: WorkerStatusSchema,
  unix_time: z.int(),
  stdout_log_path: z.string().optional(),
  stderr_log_path: z.string().optional(),
  system_log_path: z.string().optional(),
  audio_path: z.string().optional(),
});


export type WorkerStatus = z.infer<typeof WorkerStatusSchema>;
export type AudioExtension = z.infer<typeof AudioExtensionSchema>;
export type TranscodeState = z.infer<typeof TranscodeStateSchema>;
export type FfmpegRow = z.infer<typeof FfmpegRowSchema>;

export async function get_transcodes() {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_transcodes`);
  const json = await response.json();
  const state =  FfmpegRowSchema.array().parse(json);
  return state;
}

export function get_data_url(path: string): string {
  return `${BASE_URL}/data/${path}`;
}
