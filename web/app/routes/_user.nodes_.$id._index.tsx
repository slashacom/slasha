import { useParams } from 'react-router';
import { ServerMetricsView, getRoundedNow } from '~/components/nodes/metrics';
import { getNodeMetricsOptions } from '~/queries/nodes';
import { queryClient } from '~/utils/query-client';

export async function clientLoader({ params }: { params: { id: string } }) {
  const now = getRoundedNow();
  const start = new Date(now.getTime() - 1 * 3600 * 1000);
  await queryClient.ensureQueryData(
    getNodeMetricsOptions(params.id, start, now)
  );
  return null;
}

export default function NodeMetricsTab() {
  const { id } = useParams<{ id: string }>();
  return <ServerMetricsView nodeId={id!} />;
}
