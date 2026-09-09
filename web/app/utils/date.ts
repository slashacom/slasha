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

function joinUnits(
  value: number,
  unit: string,
  rest: number,
  restUnit: string
) {
  if (rest === 0) {
    return `${value}${unit}`;
  }
  return `${value}${unit} ${rest}${restUnit}`;
}

export function formatSeconds(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value) || value < 0) {
    return '—';
  }

  const seconds = Math.round(value);
  if (seconds < 60) {
    return `${seconds}s`;
  }

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return joinUnits(minutes, 'm', seconds % 60, 's');
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return joinUnits(hours, 'h', minutes % 60, 'm');
  }

  return joinUnits(Math.floor(hours / 24), 'd', hours % 24, 'h');
}

export function formatDuration(
  start: string | Date | null | undefined,
  end?: string | Date | null
) {
  const from = toValidDate(start);
  if (!from) {
    return '—';
  }

  return formatSeconds(
    differenceInSeconds(toValidDate(end) ?? new Date(), from)
  );
}

export function formatUptime(startedAt: string | Date | null | undefined) {
  return formatDuration(startedAt);
}
