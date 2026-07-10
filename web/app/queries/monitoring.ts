import { queryOptions } from '@tanstack/react-query';
import { httpGet } from '~/utils/http';
import type { ServerMetrics } from '~/models/server-metrics';

const REFRESH_INTERVAL = 15000;

export function getServerMetricsOptions(hours: number) {
  return queryOptions({
    queryKey: ['monitoring', 'metrics', hours],
    queryFn: () => {
      const end = new Date();
      const start = new Date(end.getTime() - hours * 3600 * 1000);
      const queryParams = new URLSearchParams({
        start: start.toISOString(),
        end: end.toISOString(),
      });
      return httpGet<{ metrics: ServerMetrics[] }>(
        `monitoring/metrics?${queryParams.toString()}`
      );
    },
    staleTime: REFRESH_INTERVAL,
    refetchInterval: REFRESH_INTERVAL,
  });
}

export function getLatestServerMetricOptions() {
  return queryOptions({
    queryKey: ['monitoring', 'metrics', 'latest'],
    queryFn: () =>
      httpGet<{ metric: ServerMetrics | null }>('monitoring/metrics/latest'),
    staleTime: REFRESH_INTERVAL,
    refetchInterval: REFRESH_INTERVAL,
  });
}
