const BASE_URL = `http://${window.location.host}`;
const API_URL = `${BASE_URL}/api/v1`;

export const WorkerStatus = Object.freeze({
  None: "none",
  Queued: "queued",
  Running: "running",
  Finished: "finished",
  Failed: "failed",
});

export class TranscodeApi {
  static get_download_link = (id, ext, name) => {
    let param = encodeURIComponent(name);
    return `${API_URL}/get_download_link/${id}/${ext}?name=${param}`;
  }

  static get_downloads = async () => {
    let response = await fetch(`${API_URL}/get_downloads`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static get_transcodes = async () => {
    let response = await fetch(`${API_URL}/get_transcodes`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static get_download = async (id) => {
    let response = await fetch(`${API_URL}/get_download/${id}`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static get_transcode = async (id, ext) => {
    let response = await fetch(`${API_URL}/get_transcode/${id}/${ext}`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static request_transcode = async (id, format) => {
    let response = await fetch(`${API_URL}/request_transcode/${id}/${format}`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static delete_transcode = async (id, format) => {
    let response = await fetch(`${API_URL}/delete_transcode/${id}/${format}`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static delete_download = async (id) => {
    let response = await fetch(`${API_URL}/delete_download/${id}`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static get_download_progress = async (id) => {
    let response = await fetch(`${API_URL}/get_download_state/${id}`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static get_transcode_progress = async (id, format) => {
    let response = await fetch(`${API_URL}/get_transcode_state/${id}/${format}`);
    if (!response.ok) throw response;
    return await response.json();
  }

  static get_metadata_link = (id) => {
    return `${API_URL}/get_metadata/${id}`;
  }

  static get_metadata = async (id) => {
    let response = await fetch(`${API_URL}/get_metadata/${id}`);
    if (!response.ok) throw response;
    let data = await response.json();
    if (data.items.length === 0) {
      return null;
    }
    return data.items[0];
  }
}
