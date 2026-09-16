import { Suspense } from 'react';
import { Outlet, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { getAppOptions } from '~/queries/apps';
import { getDeploymentsOptions } from '~/queries/deployments';
import { getAuthMeOptions } from '~/queries/auth';
import { PageSkeleton } from '~/components/global/page-skeleton';
import { TabNav } from '~/components/interface/tab-nav';
import { TabActionsProvider } from '~/components/interface/tab-actions';
import { queryClient } from '~/utils/query-client';
import { PageHeader } from '~/components/interface/page-header';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await Promise.all([
    queryClient.query({ ...getAuthMeOptions(), staleTime: 'static' }),
    queryClient.query({ ...getAppOptions(params.slug), staleTime: 'static' }),
    queryClient.query({
      ...getDeploymentsOptions(params.slug),
      staleTime: 'static',
    }),
  ]);
}

export function meta() {
  return [{ title: 'App \u00b7 slasha' }];
}

export default function AppLayout() {
  const { slug } = useParams();
  const { data: authData } = useSuspenseQuery(getAuthMeOptions());
  const { data } = useSuspenseQuery({
    ...getAppOptions(slug!),
    refetchInterval: (query) => {
      const runtimeStatus = query.state.data?.runtime_status;
      if (
        runtimeStatus === 'deploying' ||
        runtimeStatus === 'migrating' ||
        runtimeStatus === 'scaling' ||
        runtimeStatus === 'syncing'
      ) {
        return 2000;
      }
      return 5000;
    },
  });
  const app = data.app;
  const user = authData.user;
  const membership = data.membership;

  const isAdmin = user.role === 'Admin';
  const isOwner = isAdmin || !!membership?.is_owner;
  const canManageSettings =
    isAdmin || isOwner || !!membership?.can_manage_settings;
  const canManageMembers =
    isAdmin || isOwner || !!membership?.can_manage_members;
  const canPull = isAdmin || isOwner || !!membership?.can_pull;
  const canAccessSettings = canManageSettings || canManageMembers;

  if (!app) {
    return (
      <div className="px-8 py-6">
        <PageHeader
          title="App not found"
          description="The application you're looking for doesn't exist."
        />
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TabActionsProvider>
        {(slot) => (
          <>
            <TabNav
              className="shrink-0 bg-surface/30 px-8"
              actions={slot}
              items={[
                { label: 'Deployments', to: `/apps/${slug}/deployments` },
                { label: 'Scaling', to: `/apps/${slug}/scaling` },
                { label: 'Services', to: `/apps/${slug}/services` },
                { label: 'Crons', to: `/apps/${slug}/crons` },
                { label: 'Metrics', to: `/apps/${slug}/metrics` },
                ...(canPull
                  ? [{ label: 'Files', to: `/apps/${slug}/files` }]
                  : []),
                ...(canAccessSettings
                  ? [{ label: 'Settings', to: `/apps/${slug}/settings` }]
                  : []),
              ]}
            />

            <Suspense fallback={<PageSkeleton />}>
              <Outlet />
            </Suspense>
          </>
        )}
      </TabActionsProvider>
    </div>
  );
}
