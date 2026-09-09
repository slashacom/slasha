import { useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { getNodeOptions } from '~/queries/nodes';
import { queryClient } from '~/utils/query-client';
import { NodeConsole } from '~/components/nodes/node-console';

export async function clientLoader({ params }: { params: { id: string } }) {
  await queryClient.ensureQueryData(getNodeOptions(params.id));
  return null;
}

export default function NodeConsoleTab() {
  const { id } = useParams<{ id: string }>();
  const { data } = useSuspenseQuery(getNodeOptions(id!));

  return <NodeConsole node={data.node} />;
}
