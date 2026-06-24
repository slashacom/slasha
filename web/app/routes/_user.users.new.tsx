import { useNavigate, redirect } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { useCreateUser } from '~/queries/users';
import { getAppsOptions } from '~/queries/apps';
import { UserForm } from '~/components/users/user-form';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    return redirect('/apps');
  }
  await queryClient.ensureQueryData(getAppsOptions());
  return null;
}

export function meta() {
  return [{ title: 'Add user · slasha' }];
}

export default function NewUser() {
  const navigate = useNavigate();
  const createUser = useCreateUser();
  const { data: appsData } = useSuspenseQuery(getAppsOptions());
  const apps = appsData.apps.map((item) => item.app);

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;
    const role = formData.get('role') as string;
    const app_ids =
      role === 'User' ? (formData.getAll('app_ids') as string[]) : [];

    const promise = createUser.mutateAsync({ email, password, role, app_ids });

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
    <div>
      <div>
        <h3 className="font-semibold text-text">Add user</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Create a new account for someone on your team.
        </p>
      </div>

      <div className="mt-6">
        <UserForm
          onSubmit={handleSubmit}
          onCancel={() => navigate('/users')}
          isPending={createUser.isPending}
          apps={apps}
          submitLabel="Create user"
        />
      </div>
    </div>
  );
}
