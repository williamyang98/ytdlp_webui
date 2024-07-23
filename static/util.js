export const load_html_fragments = async () => {
  let elems = document.querySelectorAll("template[href]");
  let promises = [];
  for (let elem of elems) {
    let promise = async () => {
      let id = elem.getAttribute("id");
      let href = elem.getAttribute("href");
      let res = await fetch(href);
      let body = await res.text();
      elem.innerHTML = body;
      console.log(`Loaded HTML fragment: id=${id}, href=${href}`);
    }
    promises.push(promise());
  }
  return await Promise.all(promises);
}

export const extract_youtube_video_id = (url) => {
  const ID_REGEX = /(?:^.*(?:(?:youtu.be\/)|(?:v\/)|(?:\/u\/\w\/)|(?:embed\/)|(?:watch\?))\??v?=?)?([^#&?]*).*/;
  const ID_LENGTH = 11;
  let match = url.match(ID_REGEX);
  if (!match) { return null; }
  let id = match[1];
  if (id.length !== ID_LENGTH) { return null; }
  return id;
}

export const unix_time_to_string = (unix_time) => {
  let time = new Date(unix_time * 1000);
  let seconds = time.getSeconds();
  let minutes = time.getMinutes();
  let hours = time.getHours();
  let day = time.getDate();
  let month = time.getMonth()+1; // zero indexed month
  let year = time.getFullYear();
 
  month = String(month).padStart(2,'0');
  day = String(day).padStart(2,'0');
  hours = String(hours).padStart(2,'0');
  minutes = String(minutes).padStart(2,'0');
  seconds = String(seconds).padStart(2,'0');
  return `${year}/${month}/${day}-${hours}:${minutes}:${seconds}`;
}

export const convert_to_short_standard_prefix = (value) => {
  const SCALE = 1000;
  const PREFIXES = ["", "k", "M", "G", "T", "P", "E"];
  let scale_factor = 1;
  let scale_index = 0;
  for (let [i,_] of PREFIXES.entries()) {
    scale_index = i;
    let next_factor = scale_factor * SCALE;
    if (value < next_factor) break;
    scale_factor = next_factor;
  }
  let new_value = value / scale_factor;
  let prefix = PREFIXES[scale_index];
  return [new_value, prefix];
};

export const convert_to_long_standard_prefix = (value) => {
  const SCALE = 1000;
  const PREFIXES = ["", "kilo", "Mega", "Giga", "Tera", "Peta", "Exa"];
  let scale_factor = 1;
  let scale_index = 0;
  for (let [i,_] of PREFIXES.entries()) {
    scale_index = i;
    let next_factor = scale_factor * SCALE;
    if (value < next_factor) break;
    scale_factor = next_factor;
  }
  let new_value = value / scale_factor;
  let prefix = PREFIXES[scale_index];
  return [new_value, prefix];
};

// dhms = day hours minutes seconds
export const convert_seconds_to_dhms = (seconds) => {
  const DAY_TOTAL_SECONDS = 24*60*60;
  const HOURS_TOTAL_SECONDS = 60*60;
  const MINUTES_TOTAL_SECONDS = 60;

  let days = Math.floor(seconds / DAY_TOTAL_SECONDS);
  seconds -= days*DAY_TOTAL_SECONDS;
  let hours = Math.floor(seconds / HOURS_TOTAL_SECONDS);
  seconds -= hours*HOURS_TOTAL_SECONDS;
  let minutes = Math.floor(seconds / MINUTES_TOTAL_SECONDS);
  seconds -= minutes*MINUTES_TOTAL_SECONDS;
  return { days, hours, minutes, seconds };
};

export const convert_dhms_to_string = (dhms) => {
  let x = "";
  if (dhms.days > 0) { x += `${String(dhms.days).padStart(2, '0')}:` };
  if (dhms.days > 0 || dhms.hours > 0) x += `${String(dhms.hours).padStart(2, '0')}:`;
  x += `${String(dhms.minutes).padStart(2, '0')}:`;
  x += String(Math.round(dhms.seconds)).padStart(2, '0');
  return x;
};

export const youtube_duration_string_to_dhms = (duration) => {
  const YTDURATION_REGEX = /P(?:(\d+)D)?T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?/;
  let match = duration.match(YTDURATION_REGEX);
  if (match === null) return null;
  match = match.slice(1).map((x) => (x != null) ? x.replace(/\D/, '') : null);
  let [days,hours,minutes,seconds] = match.map((x) => parseInt(x) || 0);
  return { days, hours, minutes, seconds };
} 

export const to_title_case = (text) => {
  return `${text.charAt(0).toUpperCase()}${text.substring(1)}`
}

export const sanitise_to_filepath = (x) => {
  const ILLEGAL_REGEX = /[\/\?<>\\:\*\|"]/g;
  const CONTROL_REGEX = /[\x00-\x1f\x80-\x9f]/g;
  const RESERVED_REGEX = /^\.+$/;
  const WIN32_RESERVED_REGEX = /^(con|prn|aux|nul|com[0-9]|lpt[0-9])(\..*)?$/i;
  const WIN32_TRAILING_REGEX = /[\. ]+$/;
  return x
  .replace(ILLEGAL_REGEX, '')
  .replace(CONTROL_REGEX, '')
  .replace(RESERVED_REGEX, '')
  .replace(WIN32_RESERVED_REGEX, '')
  .replace(WIN32_TRAILING_REGEX, '');
}
