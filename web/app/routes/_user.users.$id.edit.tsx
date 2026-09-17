import { useNavigate, useParams, redirect } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getUserOptions, useUpdateUser } from '~/queries/users';
import { Page } from '~/components/global/page';
import { UserForm } from '~/components/users/user-form';
import { PageHeader } from '~/components/interface/page-header';

export async function clientLoader(args: { params: { id: string } }) {
  const { params } = args;
  const me = await queryClient.query({
    ...getAuthMeOptions(),
    staleTime: 'static',
  });
  if (me.user.role !== 'Admin') {
    return redirect('/apps');
  }
  await queryClient.query({
    ...getUserOptions(params.id),
    staleTime: 'static',
  });
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
    const password = formData.get('password') as string;

    const payload: Record<string, any> = {
      email,
      role,
    };
    if (password && password.trim().length > 0) {
      payload.password = password;
    }

    const promise = updateUser.mutateAsync(payload);

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
    <Page>
      <PageHeader
        title="Edit user"
        description={
          <>
            Update details for <span className="text-text">{user.email}</span>.
          </>
        }
      />

      <div className="mt-6">
        <UserForm
          initialData={user}
          onSubmit={handleSubmit}
          onCancel={() => navigate('/users')}
          isPending={updateUser.isPending}
          submitLabel="Save changes"
        />
      </div>
    </Page>
  );
}
