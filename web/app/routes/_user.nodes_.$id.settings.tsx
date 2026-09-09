import { useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getNodeOptions, useUpdateNode, useDeleteNode } from '~/queries/nodes';
import { NodeForm } from '~/components/nodes/node-form';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { DangerZone } from '~/components/global/danger-zone';

export default function NodeSettingsTab() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: nodeData } = useSuspenseQuery(getNodeOptions(id!));
  const updateNode = useUpdateNode();
  const deleteNode = useDeleteNode();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const handleUpdate = async (payload: any) => {
    const promise = updateNode.mutateAsync({ id: id!, payload });

    toast.promise(promise, {
      loading: 'Updating node configuration...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['nodes'] });
        navigate(`/nodes/${id}`);
        return `Node updated successfully`;
      },
      error: (err) => err.message || 'Failed to update node.',
    });
  };

  const node = nodeData.node;
  const isLocalNode = node.id === 'local';

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col">
      <div className="flex-1 overflow-y-auto px-8 py-6">
        <div className="max-w-3xl mb-12">
          <NodeForm
            initialData={node}
            onSubmit={handleUpdate}
            onCancel={() => navigate(`/nodes/${id}`)}
            isPending={updateNode.isPending}
            submitLabel="Save changes"
            isLocalNode={isLocalNode}
          />
        </div>

        {!isLocalNode && node.status !== 'Deleting' && (
          <>
            <div className="max-w-3xl">
              <DangerZone
                description="Destructive actions for this node."
                actionTitle="Delete this node"
                actionDescription="Once you delete a node, there is no going back. This will delete the node and run a teardown script. Please be certain."
                actionLabel="Delete Node"
                onAction={() => setShowDeleteConfirm(true)}
              />
            </div>

            <ConfirmationDialog
              open={showDeleteConfirm}
              onOpenChange={setShowDeleteConfirm}
              title="Delete Node"
              description={`Deleting ${node.name} is permanent. Slasha runs a teardown script on the server and removes the node.`}
              confirmLabel="Delete node"
              confirmText={node.name}
              isPending={deleteNode.isPending}
              onConfirm={() => {
                const promise = deleteNode.mutateAsync(node.id);
                toast.promise(promise, {
                  loading: 'Initiating node teardown and deletion...',
                  success: () => {
                    queryClient.invalidateQueries({ queryKey: ['nodes'] });
                    navigate(`/nodes/${node.id}/logs`);
                    return `Node ${node.name} deletion initiated successfully`;
                  },
                  error: (err) => err.message || 'Failed to delete node.',
                });
                setShowDeleteConfirm(false);
              }}
            />
          </>
        )}
      </div>
    </div>
  );
}
