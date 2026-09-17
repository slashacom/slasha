import { useNavigate, redirect } from 'react-router';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { useCreateUser } from '~/queries/users';
import { Page } from '~/components/global/page';
import { UserForm } from '~/components/users/user-form';
import { PageHeader } from '~/components/interface/page-header';

export async function clientLoader() {
  const me = await queryClient.query({
    ...getAuthMeOptions(),
    staleTime: 'static',
  });
  if (me.user.role !== 'Admin') {
    return redirect('/apps');
  }
  return null;
}

export function meta() {
  return [{ title: 'Add user · slasha' }];
}

export default function NewUser() {
  const navigate = useNavigate();
  const createUser = useCreateUser();

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;
    const role = formData.get('role') as string;

    const promise = createUser.mutateAsync({ email, password, role });

    toast.promise(promise, {
      loading: 'Creating user...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['users'] });
        navigate('/users');
        return `User ${email} created successfully`;
      },
      error: (err) => err.message || 'Failed to create user.',
    });
  };

  return (
    <Page>
      <PageHeader
        title="Add user"
        description="Create a new account for someone on your team."
      />

      <div className="mt-6">
        <UserForm
          onSubmit={handleSubmit}
          onCancel={() => navigate('/users')}
          isPending={createUser.isPending}
          submitLabel="Create user"
        />
      </div>
    </Page>
  );
}
