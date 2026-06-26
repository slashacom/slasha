import { queryOptions } from '@tanstack/react-query';
import { httpGet } from '~/utils/http';
import type { ServerMetrics } from '~/models/server-metrics';

export function getServerMetricsOptions(hours?: number) {
  return queryOptions({
    queryKey: ['monitoring', 'metrics', { hours }],
    queryFn: () =>
      httpGet<{ metrics: ServerMetrics[] }>(
        `monitoring/metrics${hours ? `?hours=${hours}` : ''}`
      ),
  });
}
