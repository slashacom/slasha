export type AppStatusTone =
  | 'live'
  | 'deploying'
  | 'failed'
  | 'idle'
  | 'migrating'
  | 'scaling'
  | 'syncing'
  | 'purging';

export type AppStatusView = {
  label: string;
  tone: AppStatusTone;
};

export function getAppStatusView(
  status:
    | 'idle'
    | 'deploying'
    | 'running'
    | 'failed'
    | 'migrating'
    | 'scaling'
    | 'syncing'
    | 'purging'
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
    case 'scaling':
      return { label: 'Scaling', tone: 'scaling' };
    case 'syncing':
      return { label: 'Syncing', tone: 'syncing' };
    case 'purging':
      return { label: 'Purging', tone: 'purging' };
    default:
      return { label: 'Idle', tone: 'idle' };
  }
}
