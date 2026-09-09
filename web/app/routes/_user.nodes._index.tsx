import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate, redirect } from 'react-router';
import { PlusIcon, Server } from 'lucide-react';
import { Page } from '~/components/global/page';
import { Button } from '~/components/interface/button';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodesOptions } from '~/queries/nodes';
import { EmptyPage } from '~/components/global/empty-page';
import { CardGrid } from '~/components/interface/card-grid';
import { NodeCard } from '~/components/nodes/node-card';
import { PageHeader } from '~/components/interface/page-header';

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
        <CardGrid className="mt-6">
          {[...nodesData.nodes]
            .sort((a, b) => {
              if (a.id === 'local') {
                return -1;
              }
              if (b.id === 'local') {
                return 1;
              }
              return a.name.localeCompare(b.name);
            })
            .map((node) => (
              <NodeCard key={node.id} node={node} />
            ))}
        </CardGrid>
      )}
    </Page>
  );
}
