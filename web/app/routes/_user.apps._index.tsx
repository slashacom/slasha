import * as React from 'react';
import { useQuery } from '@tanstack/react-query';
import { PageContainer } from '~/components/interface/page-container';
import { HStack, VStack } from '~/components/interface/stacks';
import { getAppsOptions } from '~/queries/apps';
import { AppList } from '~/components/apps/app-list';
import { Button } from '~/components/interface/button';
import { PlusIcon } from 'lucide-react';
import { queryClient } from '~/utils/query-client';
import { Skeleton } from '~/components/interface/skeleton';
import { CreateAppDialog } from '~/components/apps/create-app-dialog';

export async function clientLoader() {
  await queryClient.ensureQueryData(getAppsOptions());
}

export default function AppsIndex() {
  const [createDialogOpen, setCreateDialogOpen] = React.useState(false);
  const { data, isLoading } = useQuery(getAppsOptions());

  return (
    <PageContainer variant="center" className="py-10 text-justify">
      <VStack space={6}>
        <HStack justifyContent="between" alignItems="center">
          <VStack space={1}>
            <h1 className="text-3xl font-bold tracking-tight text-neutral-900">
              Apps
            </h1>
            <p className="text-neutral-500">
              Manage and browse your applications.
            </p>
          </VStack>
          <Button
            label="New App"
            icon={<PlusIcon className="size-4" />}
            onClick={() => setCreateDialogOpen(true)}
          />
        </HStack>

        {isLoading ? (
          <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {[...Array(6)].map((_, i) => (
              <Skeleton key={i} className="h-40 rounded-xl bg-neutral-50" />
            ))}
          </div>
        ) : (
          <AppList apps={data?.apps ?? []} />
        )}
      </VStack>

      <CreateAppDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
      />
    </PageContainer>
  );
}
