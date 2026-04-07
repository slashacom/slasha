import { useQuery } from '@tanstack/react-query';
import { Link, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { PlusIcon } from 'lucide-react';
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
    if (!confirm(`Are you sure you want to delete user ${email}?`)) {
      return;
    }

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
    <div>
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">Users</h3>
          <p className="mt-2 text-sm text-text-secondary">
            Manage who has access to this instance.
          </p>
        </div>
        <Button
          label="Add user"
          icon={<PlusIcon className="size-4" />}
          onClick={() => navigate('/users/new')}
        />
      </div>

      <div className="mt-6 overflow-x-auto">
        {isLoading ? (
          <div className="space-y-2">
            {[...Array(3)].map((_, i) => (
              <Skeleton
                key={i}
                className="h-10 w-full rounded border border-border bg-surface"
              />
            ))}
          </div>
        ) : usersData?.users.length === 0 ? (
          <p className="text-sm text-text-secondary">No users yet.</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-xs text-text-tertiary">
                <th className="pb-2 pr-4 font-medium">Email</th>
                <th className="pb-2 pr-4 font-medium">Role</th>
                <th className="pb-2 pr-4 font-medium">Created</th>
                <th className="pb-2 font-medium"></th>
              </tr>
            </thead>
            <tbody>
              {usersData?.users.map((user: User) => (
                <tr
                  key={user.id}
                  className="border-b border-border last:border-0"
                >
                  <td className="py-3 pr-4 font-medium text-text">
                    {user.email}
                  </td>
                  <td className="py-3 pr-4 text-text-secondary capitalize">
                    {user.role}
                  </td>
                  <td className="py-3 pr-4 text-text-secondary">
                    {new Date(user.created_at).toLocaleDateString()}
                  </td>
                  <td className="py-3 text-right">
                    <div className="flex items-center justify-end gap-3">
                      <Link
                        to={`/users/${user.id}/edit`}
                        className="text-xs !text-text-secondary !no-underline hover:!text-text"
                      >
                        Edit
                      </Link>
                      <button
                        onClick={() => handleDelete(user.id, user.email)}
                        disabled={deleteUser.isPending}
                        className="text-xs text-red-500 hover:underline disabled:opacity-50"
                      >
                        Delete
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
