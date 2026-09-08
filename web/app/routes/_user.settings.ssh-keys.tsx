import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { PlusIcon } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { PageHeader } from '~/components/interface/page-header';
import { getSshKeysOptions, useDeleteSshKey } from '~/queries/ssh-keys';
import { queryClient } from '~/utils/query-client';
import type { SshKey } from '~/models/ssh-key';
import { SshKeyList } from '~/components/settings/ssh-key-list';

export function meta() {
  return [{ title: 'SSH Keys' }];
}

export async function clientLoader() {
  await queryClient.ensureQueryData(getSshKeysOptions());
}

export default function SshKeys() {
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getSshKeysOptions());
  const deleteKey = useDeleteSshKey();

  const [pendingDelete, setPendingDelete] = useState<SshKey | null>(null);

  const handleConfirmDelete = async () => {
    if (!pendingDelete) {
      return;
    }

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
      <PageHeader
        title="SSH Keys"
        description="Manage public SSH keys to access your applications via Git over SSH."
        actions={
          <Button
            label="Add key"
            icon={<PlusIcon className="size-4" />}
            onClick={() => navigate('/settings/ssh-keys/new')}
          />
        }
      />

      <SshKeyList
        keys={data.keys ?? []}
        onDelete={setPendingDelete}
        onAddFirst={() => navigate('/settings/ssh-keys/new')}
      />

      <ConfirmationDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title="Delete SSH Key"
        description={`Are you sure you want to delete "${
          pendingDelete?.name ?? ''
        }"? This will immediately revoke access for this key.`}
        confirmLabel="Delete"
        onConfirm={handleConfirmDelete}
      />
    </div>
  );
}
