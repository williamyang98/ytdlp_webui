import { WorkerStatus } from "../api.js"
import { 
  convert_to_short_standard_prefix, convert_dhms_to_string, convert_seconds_to_dhms,
  unix_time_to_string,
  to_title_case
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

export const DownloadProgress = {
  props: {
    state: Object, // download entry
    progress: Object, // download progress (optional)
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
          let percentage = this.progress.percentage; // 0 to 100
          percentage = (percentage === null) ? 0 : percentage;
          return { width: percentage, class: 'bg-primary', text: `${Math.round(percentage)}%` };
        };
      }
      return { width: 100, class: 'bg-warning', text: "Unknown"};
    },
    subtitle_text() {
      if (this.progress == null) return null;
      if (this.progress.worker_status == WorkerStatus.Failed) return this.progress.fail_reason;
      if (this.progress.downloaded_bytes == null) return "Waiting for download to start";

      let [curr_bytes, curr_bytes_unit] = convert_to_short_standard_prefix(this.progress.downloaded_bytes);
      let [total_bytes, total_bytes_unit] = convert_to_short_standard_prefix(this.progress.size_bytes);
      let [speed_bytes, speed_bytes_unit] = convert_to_short_standard_prefix(this.progress.speed_bytes);
      let eta = this.progress.eta;
      let eta_string = undefined;
      if (eta !== null) {
        eta_string = `ETA ${convert_dhms_to_string(eta)}`;
      } else {
        eta_string = "Unknown estimated time";
      }
      let text_size_progress = `${curr_bytes.toFixed(2)}${curr_bytes_unit}B/${total_bytes.toFixed(2)}${total_bytes_unit}B`;
      let text_speed = `${speed_bytes.toFixed(2)}${speed_bytes_unit}B/s`;
      let text = `${text_size_progress} @ ${text_speed} - (${eta_string})`
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
      if (this.progress.percentage != null) {
        table.percentage = this.progress.percentage.toFixed(2);
        let [curr_bytes, curr_bytes_unit] = convert_to_short_standard_prefix(this.progress.downloaded_bytes);
        let [total_bytes, total_bytes_unit] = convert_to_short_standard_prefix(this.progress.size_bytes);
        table.download_size = `${curr_bytes.toFixed(2)} ${curr_bytes_unit}Bytes`;
        table.total_size = `${total_bytes.toFixed(2)} ${total_bytes_unit}bytes`;
        let [speed_bytes, speed_bytes_unit] = convert_to_short_standard_prefix(this.progress.speed_bytes);
        table.download_speed = `${speed_bytes.toFixed(2)} ${speed_bytes_unit}B/s`;
        table.eta = convert_dhms_to_string(this.progress.eta);
      }
      return table;
    },
  },
  template: document.querySelector("template#download-progress"),
};
