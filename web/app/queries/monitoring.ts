import { queryOptions } from '@tanstack/react-query';
import { httpGet } from '~/utils/http';
import type { ServerMetrics } from '~/models/server-metrics';

export function getServerMetricsOptions(
  start?: Date,
  end?: Date,
  nodeId?: string
) {
  let queryParams = new URLSearchParams();
  if (start) queryParams.append('start', start.toISOString());
  if (end) queryParams.append('end', end.toISOString());
  if (nodeId) queryParams.append('node_id', nodeId);

  const qs = queryParams.toString();

  return queryOptions({
    queryKey: ['monitoring', 'metrics', { start, end, nodeId }],
    queryFn: () =>
      httpGet<{ metrics: ServerMetrics[] }>(
        `monitoring/metrics${qs ? `?${qs}` : ''}`
      ),
  });
}
