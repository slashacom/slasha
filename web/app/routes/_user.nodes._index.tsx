import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Link, useNavigate, redirect } from 'react-router';
import { toast } from 'sonner';
import { PlusIcon, Server, Terminal, Trash2 } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { Table } from '~/components/interface/table';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import {
  getNodesOptions,
  useDeleteNode,
  type NodeWithStatus,
} from '~/queries/nodes';
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
  const localNode = nodesData.nodes.find((n) => n.id === 'local');
  const remoteNodes = nodesData.nodes.filter((n) => n.id !== 'local');
  const deleteNode = useDeleteNode();
  const [pendingDelete, setPendingDelete] = useState<NodeWithStatus | null>(
    null
  );

  const handleConfirmDelete = () => {
    if (!pendingDelete) {
      return;
    }
    const { id, name } = pendingDelete;

    const promise = deleteNode.mutateAsync(id);

    toast.promise(promise, {
      loading: 'Initiating node teardown and deletion...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['nodes'] });
        return `Node ${name} deletion initiated successfully`;
      },
      error: (err) => err.message || 'Failed to delete node.',
    });

    setPendingDelete(null);
  };

  return (
    <div>
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">Nodes</h3>
          <p className="mt-2 text-sm text-text-secondary">
            Manage the server nodes running Slasha apps and view cluster health.
          </p>
        </div>
        <Button
          label="Add Node"
          icon={<PlusIcon className="size-4" />}
          onClick={() => navigate('/nodes/new')}
        />
      </div>

      {localNode && (
        <div className="mt-6 rounded-lg border border-border bg-surface/20 p-5">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h4 className="font-semibold text-text">{localNode.name}</h4>
                <span className="rounded bg-white/[0.04] border border-white/[0.02] px-1.5 py-0.5 text-[10px] font-medium text-text-secondary">
                  Local
                </span>
              </div>
              <p className="text-[12px] text-text-secondary mt-1">
                Deploys applications directly on the host machine running
                Slasha.
              </p>
            </div>
            <div className="flex items-center">
              <NodeStatusBadge
                status={localNode.status as any}
                liveStatus={localNode.live_status}
              />
            </div>
          </div>
        </div>
      )}

      <div className="mt-8">
        <h4 className="font-semibold text-text text-sm mb-4">
          Remote Cluster Nodes
        </h4>
        {remoteNodes.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-lg border border-border border-dashed p-8 text-center bg-surface/5">
            <Server className="size-8 text-text-tertiary mb-2" />
            <p className="text-sm font-medium text-text-secondary">
              No remote nodes connected
            </p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <Table
              columns={[
                'Name',
                'Host Connection',
                'Status',
                { label: '', align: 'right' },
              ]}
            >
              {remoteNodes.map((nodeWithStatus: NodeWithStatus) => {
                const node = nodeWithStatus;
                return (
                  <tr key={node.id}>
                    <td className="py-3 pr-4 font-medium text-text">
                      {node.name}
                    </td>
                    <td className="py-3 pr-4 font-mono text-[13px] text-text-secondary">
                      {node.host
                        ? `${node.user}@${node.host}:${node.port}`
                        : '—'}
                    </td>
                    <td className="py-3 pr-4">
                      <NodeStatusBadge
                        status={node.status as any}
                        liveStatus={node.live_status}
                      />
                    </td>
                    <td className="py-3 text-right">
                      <div className="flex items-center justify-end gap-3">
                        <Link
                          to={`/nodes/${node.id}?type=${node.status === 'Deleting' ? 'teardown' : 'setup'}`}
                          className="inline-flex items-center gap-1 text-xs !text-text-secondary hover:!text-text !no-underline"
                          title="View Logs"
                        >
                          <Terminal className="size-3" />
                          Logs
                        </Link>
                        {node.status !== 'Deleting' && (
                          <button
                            onClick={() => setPendingDelete(nodeWithStatus)}
                            disabled={deleteNode.isPending}
                            className="inline-flex items-center gap-1 text-xs text-red-500 hover:text-red-400 disabled:opacity-50"
                            title="Delete Node"
                          >
                            <Trash2 className="size-3" />
                            Delete
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </Table>
          </div>
        )}
      </div>

      <ConfirmationDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title="Delete Node"
        description={
          pendingDelete
            ? `Are you sure you want to delete ${pendingDelete.name}? This will run the teardown script on the server and remove the node from Slasha. This cannot be undone.`
            : ''
        }
        confirmLabel="Teardown & Delete"
        onConfirm={handleConfirmDelete}
      />
    </div>
  );
}
