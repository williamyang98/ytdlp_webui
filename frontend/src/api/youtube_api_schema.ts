import * as z from "zod";
import { youtube_duration_string_to_dhms } from "../utility/format.ts";

export type VideoId = string;
export type PlaylistId = string;

function convert_video_id(id: string): VideoId {
  return id;
}

// mirror to /src/youtube_api/src/schema.rs
export const ThumbnailSchema = z.object({
  url: z.string(),
  width: z.int().gte(0),
  height: z.int().gte(0),
});

export const VideoContentDetailsSchema = z.object({
  duration: z.string().transform(youtube_duration_string_to_dhms),
  dimension: z.string(),
  definition: z.string(),
  caption: z.string(),
  licensedContent: z.boolean(),
});

export const VideoSnippetSchema = z.object({
  publishedAt: z.iso.datetime().transform(s => new Date(s)),
  channelId: z.string(),
  title: z.string(),
  description: z.string(),
  thumbnails: z.record(z.string(), ThumbnailSchema.optional()),
  channelTitle: z.string(),
  tags: z.string().array(),
  categoryId: z.string(),
});

export const VideoItemSchema = z.object({
  id: z.string().transform(convert_video_id),
  etag: z.string(),
  kind: z.string(),
  snippet: VideoSnippetSchema,
  contentDetails: VideoContentDetailsSchema,
});

export const PlaylistContentDetailsSchema = z.object({
  videoId: z.string().transform(convert_video_id),
  videoPublishedAt: z.iso.datetime().transform(s => new Date(s)),
});

export const PlaylistItemSchema = z.object({
  id: z.string(),
  etag: z.string(),
  kind: z.string(),
  contentDetails: PlaylistContentDetailsSchema,
});

export const PlaylistItemArraySchema = PlaylistItemSchema.array();

export type VideoItem = z.infer<typeof VideoItemSchema>;
export type PlaylistItem = z.infer<typeof PlaylistItemSchema>;
