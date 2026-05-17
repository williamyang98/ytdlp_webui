export function format_date(date: Date): string {
  const seconds = date.getSeconds();
  const minutes = date.getMinutes();
  const hours = date.getHours();
  const day = date.getDate();
  const month = date.getMonth()+1; // zero indexed month
  const year = date.getFullYear();

  const s_month = String(month).padStart(2,'0');
  const s_day = String(day).padStart(2,'0');
  const s_hours = String(hours).padStart(2,'0');
  const s_minutes = String(minutes).padStart(2,'0');
  const s_seconds = String(seconds).padStart(2,'0');
  return `${year}/${s_month}/${s_day}-${s_hours}:${s_minutes}:${s_seconds}`;
}

export function convert_to_short_standard_prefix(value: number) {
  const SCALE = 1000;
  const PREFIXES = ["", "k", "M", "G", "T", "P", "E"];
  let scale_factor = 1;
  let scale_index = 0;
  for (const [i,_] of PREFIXES.entries()) {
    scale_index = i;
    const next_factor = scale_factor * SCALE;
    if (value < next_factor) break;
    scale_factor = next_factor;
  }
  value = value / scale_factor;
  const prefix = PREFIXES[scale_index];
  return { value, prefix };
};

export function convert_to_long_standard_prefix(value: number) {
  const SCALE = 1000;
  const PREFIXES = ["", "kilo", "Mega", "Giga", "Tera", "Peta", "Exa"];
  let scale_factor = 1;
  let scale_index = 0;
  for (const [i,_] of PREFIXES.entries()) {
    scale_index = i;
    const next_factor = scale_factor * SCALE;
    if (value < next_factor) break;
    scale_factor = next_factor;
  }
  value = value / scale_factor;
  const prefix = PREFIXES[scale_index];
  return { value, prefix };
};

export interface DHMS {
  days: number;
  hours: number;
  minutes: number;
  seconds: number;
}

export function convert_seconds_to_dhms(seconds: number): DHMS {
  const DAY_TOTAL_SECONDS = 24*60*60;
  const HOURS_TOTAL_SECONDS = 60*60;
  const MINUTES_TOTAL_SECONDS = 60;

  const days = Math.floor(seconds / DAY_TOTAL_SECONDS);
  seconds -= days*DAY_TOTAL_SECONDS;
  const hours = Math.floor(seconds / HOURS_TOTAL_SECONDS);
  seconds -= hours*HOURS_TOTAL_SECONDS;
  const minutes = Math.floor(seconds / MINUTES_TOTAL_SECONDS);
  seconds -= minutes*MINUTES_TOTAL_SECONDS;
  return { days, hours, minutes, seconds };
};

export function convert_dhms_to_string(dhms: DHMS): string {
  let x = "";
  if (dhms.days > 0) { x += `${String(dhms.days).padStart(2, '0')}:` };
  if (dhms.days > 0 || dhms.hours > 0) x += `${String(dhms.hours).padStart(2, '0')}:`;
  x += `${String(dhms.minutes).padStart(2, '0')}:`;
  x += String(Math.round(dhms.seconds)).padStart(2, '0');
  return x;
};

export function youtube_duration_string_to_dhms(duration: string): DHMS {
  const YTDURATION_REGEX = /P(?:(\d+)D)?T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?/;
  const match = duration.match(YTDURATION_REGEX);
  if (match === null) {
    throw Error(`Invalid youtube duration string: ${duration}`);
  }
  let parts = match.slice(1,5) as (string | undefined)[];
  parts = parts.map(x => x !== undefined ? x.replace(/\D/, '') : undefined);
  const digits = parts.map(x => x !== undefined ? parseInt(x) : 0);
  const days = digits.at(0) || 0;
  const hours = digits.at(1) || 0;
  const minutes = digits.at(2) || 0;
  const seconds = digits.at(3) || 0;
  return { days, hours, minutes, seconds };
}

export function to_title_case(text: string): string {
  return `${text.charAt(0).toUpperCase()}${text.substring(1)}`
}

export function sanitise_to_filepath(x: string): string {
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
