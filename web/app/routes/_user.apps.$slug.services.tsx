import { useParams } from 'react-router';
import { ServicesView } from '~/components/apps/services';
import { getAppServicesOptions } from '~/queries/services';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await queryClient.query({
    ...getAppServicesOptions(params.slug),
    staleTime: 'static',
  });
}

export default function AppServicesPage() {
  const { slug } = useParams();
  return <ServicesView appSlug={slug!} />;
}
