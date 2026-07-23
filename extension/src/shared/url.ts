export function hostFromUrl(value: string) {
  try {
    return new URL(value).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

export function nextMonth(from: Date = new Date()) {
  const date = new Date(from);
  date.setMonth(date.getMonth() + 1);
  return date.toISOString().slice(0, 10);
}
