import { useSuspenseQuery } from '@tanstack/react-query';
import { Link, useNavigate, redirect } from 'react-router';
import { PlusIcon, Server, HardDrive, Network } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodesOptions } from '~/queries/nodes';
import { NodeStatusBadge } from '~/components/interface/status-badge';

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
    <div>
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">Nodes</h3>
          <p className="mt-2 text-sm text-text-secondary">
            Manage the server nodes running Slasha apps.
          </p>
        </div>
        <Button
          label="Add Node"
          icon={<PlusIcon className="size-4" />}
          onClick={() => navigate('/nodes/new')}
        />
      </div>

      {nodesData.nodes.length === 0 ? (
        <div className="mt-8 flex flex-col items-center justify-center rounded-lg border border-border border-dashed p-12 text-center bg-surface/5">
          <Server className="size-10 text-text-tertiary mb-3 animate-pulse" />
          <p className="text-sm font-medium text-text-secondary">
            No nodes configured
          </p>
        </div>
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
                    liveStatus={node.live_status}
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
                        {new Date(node.created_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                </div>
              </Link>
            ))}
        </div>
      )}
    </div>
  );
}
