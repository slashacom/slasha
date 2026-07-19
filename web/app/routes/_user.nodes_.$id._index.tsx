import { useParams } from 'react-router';
import { NodeMetricsView } from '~/components/nodes/metrics';
import { getNodeMetricsOptions } from '~/queries/nodes';
import { TIME_RANGES } from '~/utils/metrics-utils';
import { queryClient } from '~/utils/query-client';

export async function clientLoader({ params }: { params: { id: string } }) {
  await queryClient.ensureQueryData(
    getNodeMetricsOptions(params.id, TIME_RANGES[0].hours)
  );
  return null;
}

export default function NodeMetricsTab() {
  const { id } = useParams<{ id: string }>();
  return <NodeMetricsView nodeId={id!} />;
}
