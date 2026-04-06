import { useParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { getAppOptions } from '~/queries/apps';
import { PageContainer } from '~/components/interface/page-container';
import { VStack, HStack } from '~/components/interface/stacks';
import { queryClient } from '~/utils/query-client';

export async function clientLoader({ params }: { params: { slug: string } }) {
  await queryClient.ensureQueryData(getAppOptions(params.slug));
}

export default function AppIndexPage() {
  const { slug } = useParams();
  const { data, isLoading } = useQuery(getAppOptions(slug!));

  if (isLoading) return null;

  const app = data?.app;

  if (!app) {
    return (
      <PageContainer variant="center" className="py-10">
        <h1 className="text-xl font-bold">App not found</h1>
      </PageContainer>
    );
  }

  return (
    <PageContainer variant="center" className="py-20 text-justify">
      <VStack space={6}>
        <HStack justifyContent="between" alignItems="center">
          <VStack space={1}>
            <h1 className="text-4xl font-bold tracking-tight text-black">
              {app.name}
            </h1>
            <p className="font-mono text-sm text-neutral-500">{app.slug}</p>
          </VStack>
          <div className="rounded-full border border-neutral-200 bg-neutral-50 px-3 py-1 text-xs font-medium text-neutral-600">
            {app.status}
          </div>
        </HStack>

        <div className="h-px w-full bg-neutral-100" />
      </VStack>
    </PageContainer>
  );
}
