export type TimeRange = {
  label: string;
  hours: number;
};

export const TIME_RANGES: TimeRange[] = [
  { label: '1 Hour', hours: 1 },
  { label: '6 Hours', hours: 6 },
  { label: '24 Hours', hours: 24 },
  { label: '7 Days', hours: 168 },
  { label: '14 Days', hours: 336 },
  { label: '30 Days', hours: 720 },
];

export const formatBytes = (bytes: number | bigint, decimals = 1) => {
  const numBytes = Number(bytes);
  if (numBytes === 0) {
    return '0 B';
  }
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(numBytes) / Math.log(k));
  return (
    parseFloat((numBytes / Math.pow(k, i)).toFixed(decimals)) + ' ' + sizes[i]
  );
};

export const formatBps = (bytesPerSec: number | bigint, decimals = 1) => {
  return formatBytes(bytesPerSec, decimals) + '/s';
};

export const formatMiB = (mib: number | bigint) => {
  const numMib = Number(mib);
  if (numMib >= 1024) {
    return `${(numMib / 1024).toFixed(1)} GiB`;
  }
  return `${numMib} MiB`;
};
