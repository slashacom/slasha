import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { PlusIcon } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { getSshKeysOptions, useDeleteSshKey } from '~/queries/ssh-keys';
import { queryClient } from '~/utils/query-client';
import type { SshKey } from '~/models/ssh-key';
import { SshKeyList } from '~/components/settings/ssh-key-list';
import { AddSshKeyDialog } from '~/components/settings/add-ssh-key-dialog';

export function meta() {
  return [{ title: 'SSH Keys' }];
}

export async function clientLoader() {
  await queryClient.ensureQueryData(getSshKeysOptions());
}

export default function SshKeys() {
  const { data, isLoading } = useQuery(getSshKeysOptions());
  const deleteKey = useDeleteSshKey();

  const [isAddDialogOpen, setIsAddDialogOpen] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<SshKey | null>(null);

  const handleConfirmDelete = async () => {
    if (!pendingDelete) return;

    const promise = deleteKey.mutateAsync(pendingDelete.id);

    toast.promise(promise, {
      loading: 'Deleting SSH key...',
      success: 'SSH key deleted successfully',
      error: (err) => err.message || 'Failed to delete SSH key.',
    });

    try {
      await promise;
      setPendingDelete(null);
    } catch {}
  };

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">SSH Keys</h3>
          <p className="mt-2 text-sm text-text-secondary">
            Manage public SSH keys to access your applications via Git over SSH.
          </p>
        </div>
        <Button
          label="Add key"
          icon={<PlusIcon className="size-4" />}
          onClick={() => setIsAddDialogOpen(true)}
        />
      </div>

      <SshKeyList
        keys={data?.keys ?? []}
        isLoading={isLoading}
        onDelete={setPendingDelete}
        onAddFirst={() => setIsAddDialogOpen(true)}
      />

      <AddSshKeyDialog
        isOpen={isAddDialogOpen}
        onOpenChange={setIsAddDialogOpen}
      />

      <ConfirmationDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title="Delete SSH Key"
        description={`Are you sure you want to delete "${
          pendingDelete?.title || 'Untitled'
        }"? This will immediately revoke access for this key.`}
        confirmLabel="Delete"
        onConfirm={handleConfirmDelete}
      />
    </div>
  );
}
