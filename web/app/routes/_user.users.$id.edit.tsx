import { useNavigate, useParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { ArrowLeft, Loader2 } from 'lucide-react';
import { PageContainer } from '~/components/interface/page-container';
import { Button } from '~/components/interface/button';
import { VStack, HStack } from '~/components/interface/stacks';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getUserOptions, useUpdateUser } from '~/queries/users';
import { redirect } from 'react-router';
import { UserForm } from '~/components/users/user-form';

export async function clientLoader({ params }: { params: { id: string } }) {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'admin') {
    return redirect('/apps');
  }
  await queryClient.ensureQueryData(getUserOptions(params.id));
  return null;
}

export function meta() {
  return [{ title: 'Edit User | Slasha' }];
}

export default function EditUser() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: userData, isLoading } = useQuery(getUserOptions(id!));
  const updateUser = useUpdateUser(id!);

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
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

  if (isLoading || !userData) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader2 className="size-8 animate-spin text-neutral-400" />
      </div>
    );
  }

  const { user } = userData;

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
              Edit User
            </h1>
            <p className="text-neutral-500">Update details for {user.email}</p>
          </VStack>
        </HStack>

        <UserForm
          initialData={user}
          onSubmit={handleSubmit}
          onCancel={() => navigate('/users')}
          isPending={updateUser.isPending}
          submitLabel="Save Changes"
        />
      </VStack>
    </PageContainer>
  );
}
