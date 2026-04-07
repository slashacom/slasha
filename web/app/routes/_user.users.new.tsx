import { useNavigate, redirect } from 'react-router';
import { toast } from 'sonner';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { useCreateUser } from '~/queries/users';
import { UserForm } from '~/components/users/user-form';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'admin') {
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

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
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
          showPassword
          submitLabel="Create user"
        />
      </div>
    </div>
  );
}
