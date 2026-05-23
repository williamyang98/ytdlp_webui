import { type VideoId, type PlaylistId } from "../api/youtube_api_schema";

export type YoutubeUrlParseResult = {
  video_id?: VideoId,
  playlist_id?: PlaylistId,
};

const VIDEO_ID_REGEX = /^[a-zA-Z0-9\-\_]{11}$/;
const PLAYLIST_ID_REGEX = /^[a-zA-Z0-9\-\_]{11,}$/;

function convert_to_optional_null<T>(value: T | undefined): T | null {
  return value !== undefined ? value : null;
}

export function parse_youtube_url(url_string: string): YoutubeUrlParseResult {
  const result: YoutubeUrlParseResult = {};
  let url = URL.parse(url_string);
  // direct video id
  if (url === null) {
    if (VIDEO_ID_REGEX.test(url_string)) {
      result.video_id = url_string;
      return result;
    } else {
      // attempt to create valid url by prepending protocol
      url = URL.parse(`https://${url_string}`);
    }
  }
  if (url === null) return result;

  const path = url.pathname.split("/");
  const path_parent = path.at(1);
  switch (path_parent) {
    // playlist url
    case "playlist": {
      const playlist_id = url.searchParams.get("list") || convert_to_optional_null(path.at(2));
      if (playlist_id  !== null && PLAYLIST_ID_REGEX.test(playlist_id)) {
        result.playlist_id = playlist_id;
      }
      break;
    }
    // video url
    case "shorts":
    case "watch":
    case "live":
    case "e":
    case "v":
    case "s": {
      const video_id = url.searchParams.get("v") || convert_to_optional_null(path.at(2));
      if (video_id !== null && VIDEO_ID_REGEX.test(video_id)) {
        result.video_id = video_id;
      }
      const playlist_id = url.searchParams.get("list");
      if (playlist_id  !== null && PLAYLIST_ID_REGEX.test(playlist_id)) {
        result.playlist_id = playlist_id;
      }
      break;
    }
    case undefined:
      break;
    default: {
      if (VIDEO_ID_REGEX.test(path_parent)) {
        result.video_id = path_parent;
      }
    }
  }
  return result;
}

export function create_youtube_link(video_id: VideoId): string {
  return `https://youtube.com/watch?v=${video_id}`;
}

export function create_youtube_playlist_link(playlist_id: PlaylistId, video_id?: VideoId): string {
  let url = `https://youtube.com/watch?list=${playlist_id}`;
  if (video_id !== undefined) {
    url += `&v=${video_id}`
  }
  return url;
}
