import { TranscodeApi } from "../api.js";
import { youtube_duration_string_to_dhms, convert_dhms_to_string } from "../util.js";

export const Metadata = {
  props: {
    metadata: Object,
  },
  computed: {
    metadata_link() {
      return TranscodeApi.get_metadata_link(this.metadata.id);
    },
    video_link() {
      return `https://youtube.com/watch?v=${this.metadata.id}`;
    },
    thumbnail() {
      const VALID_THUMBNAILS = ["medium", "standard", "maxres", "high", "default"];
      let thumbnails = this.metadata.snippet.thumbnails;
      for (let name of VALID_THUMBNAILS) {
        let thumbnail = thumbnails[name];
        if (thumbnail !== undefined) {
          return {
            url: thumbnail.url,
            width: thumbnail.width,
            height: thumbnail.height,
          }
        }
      }
      return null;
    },
    duration() {
      return convert_dhms_to_string(youtube_duration_string_to_dhms(this.metadata.contentDetails.duration));
    },
  },
  template: document.querySelector("template#yt-metadata"),
};
