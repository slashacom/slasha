import { useNavigate, useParams, redirect } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodeOptions, useUpdateNode } from '~/queries/nodes';
import { NodeForm } from '~/components/nodes/node-form';

export async function clientLoader(args: { params: { id: string } }) {
  const { params } = args;
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    return redirect('/apps');
  }
  await queryClient.ensureQueryData(getNodeOptions(params.id));
  return null;
}

export function meta() {
  return [{ title: 'Edit node · slasha' }];
}

export default function EditNode() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: nodeData } = useSuspenseQuery(getNodeOptions(id!));
  const updateNode = useUpdateNode();

  const handleUpdate = async (payload: any) => {
    const promise = updateNode.mutateAsync({ id: id!, payload });

    toast.promise(promise, {
      loading: 'Updating node configuration...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['nodes'] });
        navigate('/nodes');
        return `Node updated successfully`;
      },
      error: (err) => err.message || 'Failed to update node.',
    });
  };

  const node = nodeData.node;
  const isLocalNode = node.id === 'local';

  return (
    <div>
      <div>
        <h3 className="font-semibold text-text">Edit node</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Update settings for <span className="text-text">{node.name}</span>.
        </p>
      </div>

      <div className="mt-6">
        <NodeForm
          initialData={node}
          onSubmit={handleUpdate}
          onCancel={() => navigate('/nodes')}
          isPending={updateNode.isPending}
          submitLabel="Save changes"
          isLocalNode={isLocalNode}
        />
      </div>
    </div>
  );
}
