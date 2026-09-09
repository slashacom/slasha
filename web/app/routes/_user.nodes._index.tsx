import { useSuspenseQuery } from '@tanstack/react-query';
import { Link, useNavigate, redirect } from 'react-router';
import { PlusIcon, Server, HardDrive, Network } from 'lucide-react';
import { Page } from '~/components/global/page';
import { Button } from '~/components/interface/button';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodesOptions } from '~/queries/nodes';
import { NodeStatusBadge } from '~/components/interface/status-badge';
import { EmptyPage } from '~/components/global/empty-page';
import { PageHeader } from '~/components/interface/page-header';
import { formatDate } from '~/utils/date';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    throw redirect('/apps');
  }
  await queryClient.ensureQueryData(getNodesOptions());
  return null;
}

export default function NodesPage() {
  const navigate = useNavigate();
  const { data: nodesData } = useSuspenseQuery({
    ...getNodesOptions(),
    refetchInterval: 3000,
  });

  return (
    <Page>
      <PageHeader
        title="Nodes"
        description="Manage the server nodes running Slasha apps."
        actions={
          <Button
            label="Add Node"
            icon={<PlusIcon className="size-4" />}
            onClick={() => navigate('/nodes/new')}
          />
        }
      />

      {nodesData.nodes.length === 0 ? (
        <EmptyPage
          className="mt-8"
          icon={Server}
          size="lg"
          title="No nodes configured."
          subtitle="Nodes are the servers your apps run on. Add one to start scheduling deployments onto it."
          actionLabel="Add node"
          actionIcon={<PlusIcon className="size-3.5" />}
          actionTo="/nodes/new"
        />
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 mt-6">
          {[...nodesData.nodes]
            .sort((a, b) => {
              if (a.id === 'local') return -1;
              if (b.id === 'local') return 1;
              return a.name.localeCompare(b.name);
            })
            .map((node) => (
              <Link
                key={node.id}
                to={`/nodes/${node.id}`}
                className="group relative flex flex-col justify-between rounded-lg border border-border bg-surface p-4 !no-underline transition-colors hover:bg-white/[0.04]"
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0">
                    <h4 className="truncate font-semibold text-[14px] text-text group-hover:text-white transition-colors">
                      {node.name}
                    </h4>
                    <span className="mt-1.5 inline-flex items-center gap-1 rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
                      {node.id === 'local' ? (
                        <>
                          <HardDrive className="size-3" />
                          <span>Local</span>
                        </>
                      ) : (
                        <>
                          <Network className="size-3" />
                          <span className="font-mono truncate max-w-[200px]">
                            {node.user}
                            <span className="font-sans text-text-tertiary">
                              @
                            </span>
                            {node.host}
                            <span className="font-sans text-text-tertiary">
                              :
                            </span>
                            {node.port}
                          </span>
                        </>
                      )}
                    </span>
                  </div>
                  <NodeStatusBadge
                    status={node.status}
                    connectionStatus={node.connection_status}
                  />
                </div>

                <div className="mt-4 border-t border-border/40 pt-4">
                  <div className="flex flex-col gap-2 text-[12px]">
                    <div className="flex items-center justify-between">
                      <span className="text-text-tertiary">OS</span>
                      <span
                        className="text-text-secondary truncate max-w-[150px] capitalize"
                        title={node.os || 'Unknown'}
                      >
                        {node.os || 'Unknown'}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-text-tertiary">Added On</span>
                      <span className="text-text-secondary">
                        {formatDate(node.created_at)}
                      </span>
                    </div>
                  </div>
                </div>
              </Link>
            ))}
        </div>
      )}
    </Page>
  );
}
