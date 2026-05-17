import {
  type Metadata as YoutubeMetadata,
  MetadataSchema as YoutubeMetadataSchema,
} from "./youtube_api_schema.ts";
import {
  type VideoId,
  type TranscodeKey,
  type AudioExtension,
  type DownloadState,
  type TranscodeState,
  type YtdlpRow,
  type FfmpegRow,
  type DeleteResponse,
  type RequestTranscodeResponse,
  DownloadStateSchema,
  TranscodeStateSchema,
  YtdlpRowSchema,
  FfmpegRowSchema,
  DeleteResponseSchema,
  RequestTranscodeResponseSchema,
} from "./ytdlp_api_schema.ts";

const BASE_URL =
  import.meta.env.DEV ?
  "http://localhost:8080" :
  `${window.location.protocol}//${window.location.host}`;
const API_URL = "api/v1";

export async function request_transcode(key: TranscodeKey): Promise<RequestTranscodeResponse> {
  const response = await fetch(`${BASE_URL}/${API_URL}/request_transcode/${key.video_id}/${key.audio_ext}`);
  const json = await response.json();
  const state =  RequestTranscodeResponseSchema.parse(json);
  return state;
}

export async function delete_download(video_id: VideoId): Promise<DeleteResponse> {
  const response = await fetch(`${BASE_URL}/${API_URL}/delete_download/${video_id}`);
  const json = await response.json();
  const state =  DeleteResponseSchema.parse(json);
  return state;
}

export async function delete_transcode(key: TranscodeKey): Promise<DeleteResponse> {
  const response = await fetch(`${BASE_URL}/${API_URL}/delete_transcode/${key.video_id}/${key.audio_ext}`);
  const json = await response.json();
  const state =  DeleteResponseSchema.parse(json);
  return state;
}

export async function get_downloads(): Promise<YtdlpRow[]> {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_downloads`);
  const json = await response.json();
  const state =  YtdlpRowSchema.array().parse(json);
  return state;
}

export async function get_transcodes(): Promise<FfmpegRow[]> {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_transcodes`);
  const json = await response.json();
  const state =  FfmpegRowSchema.array().parse(json);
  return state;
}

export async function get_download(video_id: VideoId): Promise<YtdlpRow> {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_download/${video_id}`);
  const json = await response.json();
  const state =  YtdlpRowSchema.parse(json);
  return state;
}

export async function get_transcode(key: TranscodeKey): Promise<FfmpegRow> {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_transcode/${key.video_id}/${key.audio_ext}`);
  const json = await response.json();
  const state =  FfmpegRowSchema.parse(json);
  return state;
}

export async function get_download_state(video_id: VideoId): Promise<DownloadState> {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_download_state/${video_id}`);
  const json = await response.json();
  const state =  DownloadStateSchema.parse(json);
  return state;
}

export async function get_transcode_state(key: TranscodeKey): Promise<TranscodeState> {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_transcode_state/${key.video_id}/${key.audio_ext}`);
  const json = await response.json();
  const state =  TranscodeStateSchema.parse(json);
  return state;
}

export async function get_metadata(video_id: VideoId): Promise<YoutubeMetadata> {
  const response = await fetch(`${BASE_URL}/${API_URL}/get_metadata/${video_id}`);
  const json = await response.json();
  const state =  YoutubeMetadataSchema.parse(json);
  return state;
}

export function get_data_url(relative_path: string): string {
  return `${BASE_URL}/data/${relative_path}`;
}

export function get_download_link(video_id: string, audio_ext: AudioExtension, name: string): string {
  const param_name = encodeURIComponent(name);
  return `${BASE_URL}/${API_URL}/get_download_link/${video_id}/${audio_ext}?name=${param_name}`;
}

export function get_youtube_link(video_id: VideoId): string {
  return `https://youtube.com/watch?v=${video_id}`;
}
