const BASE_URL = `http://${window.location.host}`;
const API_URL = `${BASE_URL}/api/v1`;

export class Api {
  static get_downloads = async () => {
    let response = await fetch(`${API_URL}/get_downloads`);
    return await response.json();
  }

  static get_transcodes = async () => {
    let response = await fetch(`${API_URL}/get_transcodes`);
    return await response.json();
  }

  static request_transcode = async (id, format) => {
    let response = await fetch(`${API_URL}/request_transcode/${id}/${format}`);
    return await response.json();
  }

  static delete_transcode = async (id, format) => {
    let response = await fetch(`${API_URL}/delete_transcode/${id}/${format}`);
    return await response.json();
  }

  static get_download_state = async (id, format) => {
    let response = await fetch(`${API_URL}/get_download_state/${id}/${format}`);
    return await response.json();
  }

  static get_transcode_state = async (id, format) => {
    let response = await fetch(`${API_URL}/get_transcode_state/${id}/${format}`);
    return await response.json();
  }
}
