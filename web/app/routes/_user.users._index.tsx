import { useQuery } from '@tanstack/react-query';
import { Link, redirect, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { PlusIcon, Trash2, Edit2, Shield, Calendar } from 'lucide-react';
import { PageContainer } from '~/components/interface/page-container';
import { HStack, VStack } from '~/components/interface/stacks';
import { Button } from '~/components/interface/button';
import { Skeleton } from '~/components/interface/skeleton';
import { queryClient } from '~/utils/query-client';
import { getUsersOptions, useDeleteUser } from '~/queries/users';
import type { User } from '~/models/user';

export async function clientLoader() {
  await queryClient.ensureQueryData(getUsersOptions());

  return null;
}

export default function UsersPage() {
  const navigate = useNavigate();
  const { data: usersData, isLoading } = useQuery(getUsersOptions());
  const deleteUser = useDeleteUser();

  const handleDelete = (id: string, email: string) => {
    if (!confirm(`Are you sure you want to delete user ${email}?`)) return;

    const promise = deleteUser.mutateAsync(id);

    toast.promise(promise, {
      loading: 'Deleting user...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['users'] });
        return `User ${email} deleted successfully`;
      },
      error: (err) => err.message || 'Failed to delete user.',
    });
  };

  return (
    <PageContainer variant="center" className="py-10">
      <VStack space={6}>
        <HStack justifyContent="between" alignItems="center">
          <VStack space={1}>
            <h1 className="text-3xl font-bold tracking-tight text-neutral-900">
              Users
            </h1>
            <p className="text-neutral-500">
              Manage your team and their access levels.
            </p>
          </VStack>
          <Button
            label="Add User"
            icon={<PlusIcon className="size-4" />}
            onClick={() => navigate('/users/new')}
          />
        </HStack>

        <div className="overflow-hidden rounded-xl border border-neutral-100 bg-white">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-neutral-100 bg-neutral-50/50 text-neutral-500">
              <tr>
                <th className="px-6 py-4 font-medium">User</th>
                <th className="px-6 py-4 font-medium">Role</th>
                <th className="px-6 py-4 font-medium">Created</th>
                <th className="px-6 py-4 font-medium text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-neutral-100">
              {isLoading
                ? [...Array(3)].map((_, i) => (
                    <tr key={i}>
                      <td colSpan={4} className="px-6 py-4">
                        <Skeleton className="h-10 w-full rounded-md bg-neutral-50" />
                      </td>
                    </tr>
                  ))
                : usersData?.users.map((user: User) => (
                    <tr
                      key={user.id}
                      className="group hover:bg-neutral-50/50 transition-colors"
                    >
                      <td className="px-6 py-4">
                        <HStack space={3}>
                          <div className="flex size-10 items-center justify-center rounded-full bg-neutral-100 text-neutral-600 font-semibold">
                            {user.email[0].toUpperCase()}
                          </div>
                          <VStack space={0.5}>
                            <span className="font-medium text-neutral-900">
                              {user.email}
                            </span>
                          </VStack>
                        </HStack>
                      </td>
                      <td className="px-6 py-4">
                        <div className="flex items-center gap-1.5 capitalize text-neutral-600">
                          <Shield className="size-3.5 text-neutral-400" />
                          {user.role}
                        </div>
                      </td>
                      <td className="px-6 py-4 text-neutral-500">
                        <div className="flex items-center gap-1.5">
                          <Calendar className="size-3.5 text-neutral-400" />
                          {new Date(user.created_at).toLocaleDateString()}
                        </div>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <HStack space={2} justifyContent="end">
                          <Link to={`/users/${user.id}/edit`}>
                            <Button
                              variant="ghost"
                              icon={
                                <Edit2 className="size-4 text-neutral-400 group-hover:text-neutral-900 transition-colors" />
                              }
                            />
                          </Link>
                          <Button
                            variant="ghost"
                            icon={
                              <Trash2 className="size-4 text-neutral-400 group-hover:text-red-500 transition-colors" />
                            }
                            onClick={() => handleDelete(user.id, user.email)}
                          />
                        </HStack>
                      </td>
                    </tr>
                  ))}
            </tbody>
          </table>

          {!isLoading && usersData?.users.length === 0 && (
            <div className="py-20 text-center text-neutral-500">
              No users found.
            </div>
          )}
        </div>
      </VStack>
    </PageContainer>
  );
}
