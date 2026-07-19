import { useState } from 'react';
import { useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { Server } from 'lucide-react';
import { toast } from 'sonner';
import { useMoveAppNode } from '~/queries/apps';
import { getNodesOptions } from '~/queries/nodes';
import type { App } from '~/models/app';
import { Button } from '~/components/interface/button';
import { Select } from '~/components/interface/select';
import { HStack, VStack } from '~/components/interface/stacks';

type AppNodeManagerProps = {
  app: App;
};

export function AppNodeManager(props: AppNodeManagerProps) {
  const { app } = props;
  const queryClient = useQueryClient();
  const moveAppNode = useMoveAppNode();
  const { data: nodesData } = useSuspenseQuery(getNodesOptions());
  const [nodeId, setNodeId] = useState(app.node_id);

  const handleMove = async () => {
    if (nodeId === app.node_id) {
      return;
    }

    const promise = moveAppNode.mutateAsync({
      appSlug: app.slug,
      node_id: nodeId,
    });

    toast.promise(promise, {
      loading: 'Initiating server migration...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
        queryClient.invalidateQueries({ queryKey: ['apps'] });
        return 'Server migration initiated successfully';
      },
      error: (error) => error.message || 'Failed to move app to new node.',
    });
  };

  const readyNodes =
    nodesData?.nodes?.filter((n) => n.status === 'Ready') ?? [];

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div className="px-6 py-5">
          <HStack space={3} alignItems="start">
            <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
              <Server className="size-5" />
            </div>
            <div>
              <h3 className="text-[15px] font-semibold text-text">
                App Server Node
              </h3>
              <p className="mt-0.5 text-[13px] text-text-tertiary">
                Move your application to another server node. The application
                will be redeployed on the target node.
              </p>
              <div className="mt-4 flex items-center gap-3">
                <Select
                  value={nodeId}
                  onChange={(event) => setNodeId(event.target.value)}
                  className="w-64"
                  disabled={moveAppNode.isPending}
                >
                  {readyNodes.map((n) => (
                    <option key={n.id} value={n.id}>
                      {n.name} {n.id === 'local' ? '(Local)' : `(${n.host})`}
                    </option>
                  ))}
                </Select>
                <Button
                  label="Move App"
                  size="sm"
                  onClick={handleMove}
                  disabled={
                    moveAppNode.isPending ||
                    nodeId === app.node_id ||
                    readyNodes.length === 0
                  }
                />
              </div>
            </div>
          </HStack>
        </div>
      </div>
    </VStack>
  );
}
