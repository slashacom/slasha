import { differenceInSeconds, format, isValid } from 'date-fns';

const DATE_TIME_FORMAT = 'MMM d, yyyy, h:mm a';
const DATE_FORMAT = 'MMM d, yyyy';
const TIME_FORMAT = 'h:mm:ss a';

export function parseUTC(value: string | Date): Date {
  if (value instanceof Date) {
    return value;
  }
  if (!value.endsWith('Z') && !value.includes('+')) {
    return new Date(`${value}Z`);
  }
  return new Date(value);
}

function toValidDate(value: string | Date | null | undefined): Date | null {
  if (!value) {
    return null;
  }

  const parsed = parseUTC(value);
  return isValid(parsed) ? parsed : null;
}

export function formatDateTime(value: string | Date | null | undefined) {
  const date = toValidDate(value);
  return date ? format(date, DATE_TIME_FORMAT) : '—';
}

export function formatDate(value: string | Date | null | undefined) {
  const date = toValidDate(value);
  return date ? format(date, DATE_FORMAT) : '—';
}

export function formatTime(value: string | Date | null | undefined) {
  const date = toValidDate(value);
  return date ? format(date, TIME_FORMAT) : '—';
}

export function formatRelativeTime(value: string | Date | null | undefined) {
  const date = toValidDate(value);
  if (!date) {
    return '—';
  }

  const seconds = differenceInSeconds(new Date(), date);
  if (seconds < 60) {
    return 'just now';
  }

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes}m ago`;
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ago`;
  }

  return `${Math.floor(hours / 24)}d ago`;
}

export function formatDuration(
  start: string | Date | null | undefined,
  end?: string | Date | null
) {
  const from = toValidDate(start);
  if (!from) {
    return '—';
  }

  const seconds = differenceInSeconds(toValidDate(end) ?? new Date(), from);
  if (seconds < 0) {
    return '—';
  }
  if (seconds < 60) {
    return `${seconds}s`;
  }

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes}m ${seconds % 60}s`;
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ${minutes % 60}m`;
  }

  return `${Math.floor(hours / 24)}d ${hours % 24}h`;
}

export function formatUptime(startedAt: string | Date | null | undefined) {
  return formatDuration(startedAt);
}
