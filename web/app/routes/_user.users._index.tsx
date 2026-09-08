import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Pencil, PlusIcon, Trash2, Users } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { Table } from '~/components/interface/table';
import { redirect } from 'react-router';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { getUsersOptions, useDeleteUser } from '~/queries/users';
import type { User } from '~/models/user';
import { PageHeader } from '~/components/interface/page-header';
import { formatDate } from '~/utils/date';
import { TableRowActions } from '~/components/interface/table-row-actions';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    return redirect('/apps');
  }
  await queryClient.ensureQueryData(getUsersOptions());
  return null;
}

export default function UsersPage() {
  const navigate = useNavigate();
  const { data: usersData } = useSuspenseQuery(getUsersOptions());
  const deleteUser = useDeleteUser();
  const [pendingDelete, setPendingDelete] = useState<User | null>(null);

  const handleConfirmDelete = () => {
    if (!pendingDelete) {
      return;
    }
    const { id, email } = pendingDelete;

    const promise = deleteUser.mutateAsync(id);

    toast.promise(promise, {
      loading: 'Deleting user...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['users'] });
        return `User ${email} deleted successfully`;
      },
      error: (err) => err.message || 'Failed to delete user.',
    });

    setPendingDelete(null);
  };

  return (
    <div>
      <PageHeader
        title="Users"
        description="Manage who has access to this instance."
        actions={
          <Button
            label="Add user"
            icon={<PlusIcon className="size-4" />}
            onClick={() => navigate('/users/new')}
          />
        }
      />

      <div className="mt-6 overflow-x-auto">
        {usersData.users.length === 0 ? (
          <EmptyPage icon={Users} title="No users yet." />
        ) : (
          <Table
            columns={[
              'Email',
              'Role',
              'Created',
              { label: '', align: 'right' },
            ]}
          >
            {usersData.users.map((user: User) => (
              <tr key={user.id}>
                <td className="py-3 pr-4 font-medium text-text">
                  {user.email}
                </td>
                <td className="py-3 pr-4 text-text-secondary capitalize">
                  {user.role}
                </td>
                <td className="py-3 pr-4 text-text-secondary">
                  {formatDate(user.created_at)}
                </td>
                <td className="py-3 text-right">
                  <TableRowActions
                    actions={[
                      {
                        label: 'Edit user',
                        icon: Pencil,
                        to: `/users/${user.id}/edit`,
                      },
                      {
                        label: 'Delete user',
                        icon: Trash2,
                        isDestructive: true,
                        isDisabled: deleteUser.isPending,
                        onClick: () => setPendingDelete(user),
                      },
                    ]}
                  />
                </td>
              </tr>
            ))}
          </Table>
        )}
      </div>

      <ConfirmationDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title="Delete user"
        description={
          pendingDelete
            ? `Are you sure you want to delete ${pendingDelete.email}? This cannot be undone.`
            : ''
        }
        confirmLabel="Delete"
        onConfirm={handleConfirmDelete}
      />
    </div>
  );
}
