import { useNavigate, useParams, redirect } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getUserOptions, useUpdateUser } from '~/queries/users';
import { UserForm } from '~/components/users/user-form';

export async function clientLoader(args: { params: { id: string } }) {
  const { params } = args;
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'admin') {
    return redirect('/apps');
  }
  await queryClient.ensureQueryData(getUserOptions(params.id));
  return null;
}

export function meta() {
  return [{ title: 'Edit user · slasha' }];
}

export default function EditUser() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: userData } = useSuspenseQuery(getUserOptions(id!));
  const updateUser = useUpdateUser(id!);

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const email = formData.get('email') as string;
    const role = formData.get('role') as string;

    const promise = updateUser.mutateAsync({ email, role });

    toast.promise(promise, {
      loading: 'Updating user...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['users'] });
        navigate('/users');
        return `User updated successfully`;
      },
      error: (err) => err.message || 'Failed to update user.',
    });
  };

  const { user } = userData;

  return (
    <div>
      <div>
        <h3 className="font-semibold text-text">Edit user</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Update details for <span className="text-text">{user.email}</span>.
        </p>
      </div>

      <div className="mt-6">
        <UserForm
          initialData={user}
          onSubmit={handleSubmit}
          onCancel={() => navigate('/users')}
          isPending={updateUser.isPending}
          submitLabel="Save changes"
        />
      </div>
    </div>
  );
}
