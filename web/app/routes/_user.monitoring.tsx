import { ServerMetricsView } from '~/components/monitoring/metrics';
import { queryClient } from '~/utils/query-client';
import { getNodesOptions } from '~/queries/nodes';

export async function clientLoader() {
  await Promise.all([queryClient.ensureQueryData(getNodesOptions())]);
}

export default function MonitoringPage() {
  return <ServerMetricsView />;
}
