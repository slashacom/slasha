export type TimeRange = {
  label: string;
  hours: number;
};

export const TIME_RANGES: TimeRange[] = [
  { label: '1 Hour', hours: 1 },
  { label: '6 Hours', hours: 6 },
  { label: '24 Hours', hours: 24 },
  { label: '7 Days', hours: 168 },
];

export const formatBytes = (bytes: number, decimals = 1) => {
  if (bytes === 0) {
    return '0 B';
  }
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return (
    parseFloat((bytes / Math.pow(k, i)).toFixed(decimals)) + ' ' + sizes[i]
  );
};

export const formatBps = (bytesPerSec: number, decimals = 1) => {
  return formatBytes(bytesPerSec, decimals) + '/s';
};

export const formatMiB = (mib: number) => {
  if (mib >= 1024) {
    return `${(mib / 1024).toFixed(1)} GiB`;
  }
  return `${mib} MiB`;
};
