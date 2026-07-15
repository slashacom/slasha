import { useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Settings } from 'lucide-react';
import { queryClient } from '~/utils/query-client';
import { getNodeOptions, useUpdateNode, useDeleteNode } from '~/queries/nodes';
import { NodeForm } from '~/components/nodes/node-form';
import { SectionHeader } from '~/components/interface/section-header';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';

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
      <SectionHeader icon={Settings} title="Settings" />
      <div className="flex-1 overflow-y-auto p-8">
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
              <h3 className="text-[14px] font-semibold text-text">
                Danger Zone
              </h3>
              <p className="mt-1 text-[13px] text-text-tertiary">
                Destructive actions for this node.
              </p>

              <div className="mt-6 rounded-lg border border-red-500/20 bg-red-500/5 p-6">
                <div className="flex items-center justify-between gap-6">
                  <div>
                    <h4 className="text-[13px] font-medium text-red-500">
                      Delete this node
                    </h4>
                    <p className="mt-1 text-[12px] text-red-500/70">
                      Once you delete a node, there is no going back. This will
                      delete the node and run a teardown script. Please be
                      certain.
                    </p>
                  </div>
                  <Button
                    label="Delete Node"
                    color="error"
                    size="sm"
                    className="shrink-0"
                    onClick={() => setShowDeleteConfirm(true)}
                  />
                </div>
              </div>
            </div>

            <ConfirmationDialog
              open={showDeleteConfirm}
              onOpenChange={setShowDeleteConfirm}
              title="Delete Node"
              description={`Are you sure you want to delete ${node.name}? This action cannot be undone. This will delete the node and run a teardown script on the server.`}
              confirmLabel="Delete Node"
              onConfirm={() => {
                const promise = deleteNode.mutateAsync(node.id);
                toast.promise(promise, {
                  loading: 'Initiating node teardown and deletion...',
                  success: () => {
                    queryClient.invalidateQueries({ queryKey: ['nodes'] });
                    navigate('/nodes');
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
