import type { Deployment } from '~/models/deployment';
import type { AppStatus } from '~/models/app';
import { parseUTC } from '~/utils/format';

export type AppStatusTone = 'live' | 'deploying' | 'failed' | 'idle';

export type AppStatusView = {
  label: string;
  tone: AppStatusTone;
};

export function getAppStatusView(status: AppStatus): AppStatusView {
  switch (status) {
    case 'running':
      return { label: 'Live', tone: 'live' };
    case 'building':
      return { label: 'Deploying', tone: 'deploying' };
    case 'failed':
      return { label: 'Failed', tone: 'failed' };
    default:
      return { label: 'Idle', tone: 'idle' };
  }
}

export function deriveAppStatus(deployments: Deployment[]): AppStatusView {
  if (deployments.some((d) => d.status === 'Running')) {
    return { label: 'Live', tone: 'live' };
  }

  const latest = [...deployments].sort(
    (a, b) =>
      parseUTC(b.created_at).getTime() - parseUTC(a.created_at).getTime()
  )[0];

  if (!latest) {
    return { label: 'No deploys', tone: 'idle' };
  }
  if (latest.status === 'Building' || latest.status === 'Pending') {
    return { label: 'Deploying', tone: 'deploying' };
  }
  if (latest.status === 'Failed') {
    return { label: 'Failed', tone: 'failed' };
  }
  return { label: 'Idle', tone: 'idle' };
}
