import { queryOptions } from '@tanstack/react-query';
import { httpGet } from '~/utils/http';
import type { ServerMetrics } from '~/models/server-metrics';

export function getServerMetricsOptions(start?: Date, end?: Date) {
  let queryParams = new URLSearchParams();
  if (start) queryParams.append('start', start.toISOString());
  if (end) queryParams.append('end', end.toISOString());
  const qs = queryParams.toString();

  return queryOptions({
    queryKey: ['monitoring', 'metrics', { start, end }],
    queryFn: () =>
      httpGet<{ metrics: ServerMetrics[] }>(
        `monitoring/metrics${qs ? `?${qs}` : ''}`
      ),
  });
}
