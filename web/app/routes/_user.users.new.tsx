import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { ArrowLeft } from 'lucide-react';
import { PageContainer } from '~/components/interface/page-container';
import { Button } from '~/components/interface/button';
import { VStack, HStack } from '~/components/interface/stacks';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { useCreateUser } from '~/queries/users';
import { redirect } from 'react-router';
import { UserForm } from '~/components/users/user-form';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'admin') {
    return redirect('/apps');
  }
  return null;
}

export function meta() {
  return [{ title: 'Add User | Slasha' }];
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
    <PageContainer variant="center" className="py-10">
      <VStack space={6} className="max-w-2xl mx-auto">
        <HStack space={4} alignItems="center">
          <Button
            variant="ghost"
            icon={<ArrowLeft className="size-4" />}
            onClick={() => navigate('/users')}
          />
          <VStack space={1}>
            <h1 className="text-3xl font-bold tracking-tight text-neutral-900">
              Add User
            </h1>
            <p className="text-neutral-500">
              Create a new user account for your platform.
            </p>
          </VStack>
        </HStack>

        <UserForm
          onSubmit={handleSubmit}
          onCancel={() => navigate('/users')}
          isPending={createUser.isPending}
          showPassword
          submitLabel="Create User"
        />
      </VStack>
    </PageContainer>
  );
}
