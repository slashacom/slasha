import { Suspense } from 'react';
import { Outlet, useParams, useNavigate, redirect } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Server, HardDrive, Network } from 'lucide-react';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodeOptions } from '~/queries/nodes';
import { TabNav } from '~/components/interface/tab-nav';
import { queryClient } from '~/utils/query-client';
import { NodeStatusBadge } from '~/components/interface/status-badge';

export async function clientLoader(args: { params: { id: string } }) {
  const { params } = args;
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    throw redirect('/apps');
  }
  await queryClient.ensureQueryData(getNodeOptions(params.id));
  return null;
}

export default function NodeDetailLayout() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const { data: nodeData } = useSuspenseQuery({
    ...getNodeOptions(id!),
    refetchInterval: (query) => {
      const node = query.state.data?.node;
      if (node && (node.status === 'SettingUp' || node.status === 'Deleting')) {
        return 2000;
      }
      return 5000;
    },
  });
  const node = nodeData.node;

  return (
    <div className="flex flex-1 flex-col min-h-0">
      <div className="flex shrink-0 items-center justify-between gap-4 border-b border-border px-8 py-3">
        <div className="flex min-w-0 items-center gap-3">
          <Server className="size-4 shrink-0 text-text-tertiary" />
          <span className="truncate text-[13px] font-medium text-text">
            {node.name}
          </span>
          <span className="inline-flex items-center gap-1 rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
            {node.id === 'local' ? (
              <>
                <HardDrive className="size-3" />
                <span>Local</span>
              </>
            ) : (
              <>
                <Network className="size-3" />
                <span className="font-mono">
                  {node.user}
                  <span className="font-sans text-text-tertiary">@</span>
                  {node.host}
                  <span className="font-sans text-text-tertiary">:</span>
                  {node.port}
                </span>
              </>
            )}
          </span>
          <NodeStatusBadge status={node.status} liveStatus={node.live_status} />
        </div>
      </div>

      <TabNav
        className="shrink-0 bg-surface/30 px-8"
        items={[
          { label: 'Metrics', to: `/nodes/${id}`, end: true },
          ...(node.id !== 'local'
            ? [{ label: 'Logs', to: `/nodes/${id}/logs` }]
            : []),
          { label: 'Settings', to: `/nodes/${id}/settings` },
        ]}
      />

      <Suspense fallback={null}>
        <Outlet />
      </Suspense>
    </div>
  );
}
