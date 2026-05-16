export function unix_time_to_string(unix_time: number): string {
  const time = new Date(unix_time * 1000);
  const seconds = time.getSeconds();
  const minutes = time.getMinutes();
  const hours = time.getHours();
  const day = time.getDate();
  const month = time.getMonth()+1; // zero indexed month
  const year = time.getFullYear();

  const s_month = String(month).padStart(2,'0');
  const s_day = String(day).padStart(2,'0');
  const s_hours = String(hours).padStart(2,'0');
  const s_minutes = String(minutes).padStart(2,'0');
  const s_seconds = String(seconds).padStart(2,'0');
  return `${year}/${s_month}/${s_day}-${s_hours}:${s_minutes}:${s_seconds}`;
}
