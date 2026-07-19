export type AppStatusTone =
  | 'live'
  | 'deploying'
  | 'failed'
  | 'idle'
  | 'migrating';

export type AppStatusView = {
  label: string;
  tone: AppStatusTone;
};

export function getAppStatusView(
  status: 'idle' | 'deploying' | 'running' | 'failed' | 'migrating'
): AppStatusView {
  switch (status) {
    case 'running':
      return { label: 'Live', tone: 'live' };
    case 'deploying':
      return { label: 'Deploying', tone: 'deploying' };
    case 'failed':
      return { label: 'Failed', tone: 'failed' };
    case 'migrating':
      return { label: 'Migrating', tone: 'migrating' };
    default:
      return { label: 'Idle', tone: 'idle' };
  }
}
