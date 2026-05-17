import * as z from "zod";
import { youtube_duration_string_to_dhms } from "../utility/format.ts";

const ThumbnailSchema = z.object({
  url: z.string(),
  width: z.int().gte(0),
  height: z.int().gte(0),
});

const ContentDetailsSchema = z.object({
  duration: z.string().transform(youtube_duration_string_to_dhms),
  dimension: z.string(),
  definition: z.string(),
  caption: z.string(),
  licensedContent: z.boolean(),
});

const SnippetSchema = z.object({
  publishedAt: z.string(),
  channelId: z.string(),
  title: z.string(),
  description: z.string(),
  thumbnails: z.record(z.string(), ThumbnailSchema.optional()),
  channelTitle: z.string(),
  tags: z.string().array(),
  categoryId: z.string(),
});

const ItemSchema = z.object({
  id: z.string(),
  etag: z.string(),
  kind: z.string(),
  snippet: SnippetSchema,
  contentDetails: ContentDetailsSchema,
});

const PageInfoSchema = z.object({
  totalResults: z.int(),
  resultsPerPage: z.int(),
});

export const MetadataSchema = z.object({
  kind: z.string(),
  etag: z.string(),
  items: ItemSchema.array().default([]),
  pageInfo: PageInfoSchema,
});

export type Metadata = z.infer<typeof MetadataSchema>;
