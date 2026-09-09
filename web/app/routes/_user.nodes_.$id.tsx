import { Suspense, useEffect } from 'react';
import { Outlet, useParams, useNavigate, redirect } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodeOptions } from '~/queries/nodes';
import { PageSkeleton } from '~/components/global/page-skeleton';
import { TabNav } from '~/components/interface/tab-nav';
import { TabActionsProvider } from '~/components/interface/tab-actions';
import { queryClient } from '~/utils/query-client';
import { toast } from 'sonner';

export async function clientLoader(args: { params: { id: string } }) {
  const { params } = args;
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    throw redirect('/apps');
  }
  try {
    await queryClient.ensureQueryData(getNodeOptions(params.id));
  } catch (err: any) {
    if (err?.status === 404) throw redirect('/nodes');
    throw err;
  }
  return null;
}

export default function NodeDetailLayout() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const {
    data: nodeData,
    isError,
    error,
  } = useQuery({
    ...getNodeOptions(id!),
    throwOnError: false,
    refetchInterval: (query) => {
      if (query.state.status === 'error') return false;
      const node = query.state.data?.node;
      if (node && (node.status === 'SettingUp' || node.status === 'Deleting')) {
        return 2000;
      }
      return 5000;
    },
  });

  const node = nodeData?.node;
  const isDeleted = isError && (error as any)?.status === 404;

  useEffect(() => {
    if (isDeleted) {
      toast.success('Node deleted successfully.');
      navigate('/nodes');
    }
  }, [isDeleted, navigate]);

  if (!node) return null;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TabActionsProvider>
        {(slot) => (
          <>
            <TabNav
              className="shrink-0 bg-surface/30 px-8"
              actions={slot}
              items={[
                { label: 'Metrics', to: `/nodes/${id}`, end: true },
                { label: 'Console', to: `/nodes/${id}/console` },
                ...(node.id !== 'local'
                  ? [{ label: 'Logs', to: `/nodes/${id}/logs` }]
                  : []),
                { label: 'Settings', to: `/nodes/${id}/settings` },
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
