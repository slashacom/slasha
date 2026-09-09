import { useState } from 'react';
import { useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { Server } from 'lucide-react';
import { toast } from 'sonner';
import { useMoveAppNode } from '~/queries/apps';
import { getNodesOptions } from '~/queries/nodes';
import type { App } from '~/models/app';
import { Button } from '~/components/interface/button';
import { Select } from '~/components/interface/select';
import { HStack } from '~/components/interface/stacks';
import { SettingsCard } from '~/components/interface/settings-card';

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
    <SettingsCard
      icon={Server}
      title="App Server Node"
      description="Move your application to another server node. The application will be redeployed on the target node."
    >
      <HStack space={3}>
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
          onClick={handleMove}
          disabled={
            moveAppNode.isPending ||
            nodeId === app.node_id ||
            readyNodes.length === 0
          }
        />
      </HStack>
    </SettingsCard>
  );
}
