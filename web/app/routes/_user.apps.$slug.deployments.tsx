import { useParams } from 'react-router';
import { DeploymentsView } from '~/components/apps/deployments';
import { getDeploymentsOptions } from '~/queries/deployments';
import { getAppOptions } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';
import { useSuspenseQuery } from '@tanstack/react-query';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(getDeploymentsOptions(params.slug)),
    queryClient.ensureQueryData(getAppOptions(params.slug)),
  ]);
}

export default function AppDeploymentsPage() {
  const { slug } = useParams();
  const { data } = useSuspenseQuery(getAppOptions(slug!));
  return <DeploymentsView app={data.app} />;
}
