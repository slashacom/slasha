import { useNavigate, redirect } from 'react-router';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { useCreateNode } from '~/queries/nodes';
import { NodeForm } from '~/components/nodes/node-form';
import { PageHeader } from '~/components/interface/page-header';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    throw redirect('/apps');
  }
  return null;
}

export default function NewNodePage() {
  const navigate = useNavigate();
  const createNode = useCreateNode();

  const handleSubmit = async (payload: any) => {
    const promise = createNode.mutateAsync(payload);

    toast.promise(promise, {
      loading: 'Probing connection and creating node...',
      success: 'Initiating server setup.',
      error: (err) => err.message || 'Failed to connect/create node.',
    });

    try {
      const data = await promise;
      void queryClient.invalidateQueries({ queryKey: ['nodes'] });
      navigate(`/nodes/${data.node.id}/logs`);
    } catch {}
  };

  return (
    <div>
      <PageHeader
        title="Connect Node"
        description="Connect a remote node to use as a server for app deployments. Slasha will connect via SSH and automatically provision it."
      />

      <div className="mt-6">
        <NodeForm
          onSubmit={handleSubmit}
          onCancel={() => navigate('/nodes')}
          isPending={createNode.isPending}
          submitLabel="Connect Node"
          isLocalNode={false}
        />
      </div>
    </div>
  );
}
