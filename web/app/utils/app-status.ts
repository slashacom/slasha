import type { Deployment } from '~/models/deployment';
import { parseUTC } from '~/utils/format';

export type AppRuntimeTone = 'live' | 'deploying' | 'failed' | 'idle';

export type AppRuntimeStatus = {
  label: string;
  tone: AppRuntimeTone;
};

export function statusFromRuntime(runtimeStatus: string): AppRuntimeStatus {
  switch (runtimeStatus) {
    case 'running':
      return { label: 'Live', tone: 'live' };
    case 'deploying':
      return { label: 'Deploying', tone: 'deploying' };
    case 'failed':
      return { label: 'Failed', tone: 'failed' };
    default:
      return { label: 'Idle', tone: 'idle' };
  }
}

export function deriveAppStatus(deployments: Deployment[]): AppRuntimeStatus {
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
