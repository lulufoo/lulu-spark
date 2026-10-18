/** `Oct 13`, or `Oct 13, 2025` when `date` is not in the year of `now`. */
export function formatDayHeading(date: Date, now: Date = new Date()): string {
  const withYear = date.getFullYear() !== now.getFullYear();
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    ...(withYear ? { year: 'numeric' } : {}),
  });
}
