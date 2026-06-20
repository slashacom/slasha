import { useParams } from 'react-router';
import { DeploymentsView } from '~/components/apps/deployments';
import { getDeploymentsOptions } from '~/queries/deployments';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await queryClient.ensureQueryData(getDeploymentsOptions(params.slug));
}

export default function AppDeploymentsPage() {
  const { slug } = useParams();
  return <DeploymentsView appSlug={slug!} />;
}
