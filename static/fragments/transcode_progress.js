import { WorkerStatus } from "../api.js"
import { 
  convert_to_short_standard_prefix, convert_seconds_to_dhms, convert_dhms_to_string,
  unix_time_to_string,
  to_title_case,
} from "../util.js";

const status_to_colour_class = (status) => {
  switch (status) {
    case WorkerStatus.None: return "";
    case WorkerStatus.Queued: return "";
    case WorkerStatus.Running: return "bg-primary";
    case WorkerStatus.Failed: return "bg-danger";
    case WorkerStatus.Finished: return "bg-success";
  }
}

export const TranscodeProgress = {
  props: {
    state: Object,
    progress: Object,
    showTable: Boolean,
  },
  data() {
    return {}
  },
  methods: {},
  computed: {
    is_finished() {
      if (this.state.status != WorkerStatus.Finished) return false;
      if (this.progress == null) return false;
      if (this.progress.worker_status == WorkerStatus.Finished) {
        return true;
      }
      return false;
    },
    progress_bar() {
      if (this.progress == null) {
        let is_cached = (this.state.status == WorkerStatus.Finished) || (this.state.status == WorkerStatus.Failed);
        return {
          width: (this.state.status == WorkerStatus.Running) ? 0 : 100,
          class: status_to_colour_class(this.state.status),
          text: `${to_title_case(this.state.status)}${is_cached ? ' (cached)' : ''}`,
        };
      }
      switch (this.progress.worker_status) {
        case WorkerStatus.None:
        case WorkerStatus.Queued:
        case WorkerStatus.Failed:
        case WorkerStatus.Finished: {
          return {
            width: 100,
            class: status_to_colour_class(this.progress.worker_status), 
            text: to_title_case(this.progress.worker_status),
          };
        };
        case WorkerStatus.Running: {
          let percentage = 0;
          if (this.progress.transcode_duration_milliseconds != undefined && this.progress.source_duration_milliseconds != undefined) {
            percentage = this.progress.transcode_duration_milliseconds / this.progress.source_duration_milliseconds;
            percentage *= 100;
          }
          return { width: percentage, class: 'bg-primary', text: `${Math.round(Number(percentage))}%` };
        };
      }
      return { width: 100, class: 'bg-warning', text: "Unknown"};
    },
    subtitle_text() {
      if (this.progress == null) return null;
      if (this.progress.worker_status == WorkerStatus.Failed) return this.progress.fail_reason;
      if (this.progress.transcode_duration_milliseconds == null) return "Waiting for transcode to start";
      // Bitrate doesn't tell us anything useful about progress
      // let [speed_bits, speed_bits_unit] = convert_to_short_standard_prefix(this.progress.source_speed_bits);
      // let [speed_bits, speed_bits_unit] = convert_to_short_standard_prefix(this.progress.transcode_speed_bits);
      // speed_bits = Number(speed_bits).toFixed(2);

      // estimate eta given elapsed time and percentage
      let time_elapsed_seconds = this.progress.end_time_unix-this.progress.start_time_unix;
      let percentage = this.progress.transcode_duration_milliseconds / this.progress.source_duration_milliseconds;
      let remaining_percentage = 1 - percentage;
      let eta_seconds = (time_elapsed_seconds/percentage)*remaining_percentage;

      // estimate size of final file
      let estimated_total_bytes = this.progress.transcode_size_bytes/percentage;
      let [curr_bytes, curr_bytes_unit] = convert_to_short_standard_prefix(this.progress.transcode_size_bytes);
      let [total_bytes, total_bytes_unit] = convert_to_short_standard_prefix(estimated_total_bytes);

      // estimate speed of transcode
      let estimated_speed_bytes = (time_elapsed_seconds) == 0 ? 0 : this.progress.transcode_size_bytes / time_elapsed_seconds;
      let [speed_bytes, speed_bytes_unit] = convert_to_short_standard_prefix(estimated_speed_bytes);

      let eta_string = `ETA ${convert_dhms_to_string(convert_seconds_to_dhms(eta_seconds))}`;
      let curr_time_string = convert_dhms_to_string(convert_seconds_to_dhms(this.progress.transcode_duration_milliseconds/1000));
      let total_time_string = convert_dhms_to_string(convert_seconds_to_dhms(this.progress.source_duration_milliseconds/1000));

      let text_time_progress = `${curr_time_string}/${total_time_string}`;
      let text_size_progress = `${curr_bytes.toFixed(2)}${curr_bytes_unit}B/${total_bytes.toFixed(2)}${total_bytes_unit}B`;
      let text_speed = `${speed_bytes.toFixed(2)}${speed_bytes_unit}B/s`;
      let text = `${text_time_progress} - ${text_size_progress} @ ${text_speed} (${eta_string})`
      return text;
    },
    table_information() {
      if (this.progress == null) return null;
      let status = this.progress.worker_status;
      if ((status == WorkerStatus.None) || (status == WorkerStatus.Queued)) return null;
      let table = {};
      table.start_time = unix_time_to_string(this.progress.start_time_unix);
      table.end_time = unix_time_to_string(this.progress.end_time_unix);
      let elapsed_time = this.progress.end_time_unix-this.progress.start_time_unix;
      table.elapsed_time = convert_dhms_to_string(convert_seconds_to_dhms(elapsed_time));
      if (this.progress.source_duration_milliseconds != null) {
        table.source_length = convert_dhms_to_string(convert_seconds_to_dhms(this.progress.source_duration_milliseconds/1000));
        let [bitrate, bitrate_units] = convert_to_short_standard_prefix(this.progress.source_speed_bits);
        table.source_bitrate = `${bitrate.toFixed(2)} ${bitrate_units}b/s`;
      }
      if (this.progress.transcode_duration_milliseconds != null) {
        table.transcode_length = convert_dhms_to_string(convert_seconds_to_dhms(this.progress.transcode_duration_milliseconds/1000));
        let [size_bytes, size_bytes_unit] = convert_to_short_standard_prefix(this.progress.transcode_size_bytes);
        table.transcode_size = `${size_bytes.toFixed(2)} ${size_bytes_unit}bytes`;
        let [bitrate, bitrate_units] = convert_to_short_standard_prefix(this.progress.transcode_speed_bits);
        table.transcode_bitrate = `${bitrate.toFixed(2)} ${bitrate_units}b/s`;
        table.transcode_speed_factor = this.progress.transcode_speed_factor;
      }
      return table;
    },
  },
  template: document.querySelector("template#transcode-progress"),
};
