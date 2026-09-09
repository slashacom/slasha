import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { PlusIcon } from 'lucide-react';
import { toast } from 'sonner';
import { Page } from '~/components/global/page';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { TabActions } from '~/components/interface/tab-actions';
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
    <Page className="space-y-6">
      <TabActions>
        <Button
          label="Add key"
          size="sm"
          icon={<PlusIcon className="size-3.5" />}
          onClick={() => navigate('/settings/ssh-keys/new')}
        />
      </TabActions>

      <p className="max-w-prose text-pretty text-sm text-text-secondary">
        Public keys that can reach your applications over Git via SSH.
      </p>

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
    </Page>
  );
}
