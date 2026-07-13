import { useNavigate, redirect } from 'react-router';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { useCreateNode } from '~/queries/nodes';
import { NodeForm } from '~/components/nodes/node-form';

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
      success: 'Node record created successfully. Initiating server setup.',
      error: (err) => err.message || 'Failed to connect/create node.',
    });

    try {
      const data = await promise;
      void queryClient.invalidateQueries({ queryKey: ['nodes'] });
      navigate(`/nodes/${data.node.id}?type=setup`);
    } catch {}
  };

  return (
    <div>
      <div>
        <h3 className="font-semibold text-text">Connect Node</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Add a remote Docker host to your cluster. Slasha will connect via SSH
          and automatically provision the server.
        </p>
      </div>

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
