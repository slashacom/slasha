import { useState } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { PlusIcon } from 'lucide-react';
import { toast } from 'sonner';
import { Page } from '~/components/global/page';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { TabActions } from '~/components/interface/tab-actions';
import { getS3StoragesOptions, useDeleteS3Storage } from '~/queries/s3-storage';
import { queryClient } from '~/utils/query-client';
import type { S3Storage } from '~/models/s3-storage';
import { S3StorageList } from '~/components/settings/s3-storage-list';

export function meta() {
  return [{ title: 'S3 Storages · slasha' }];
}

export async function clientLoader() {
  await queryClient.query({ ...getS3StoragesOptions(), staleTime: 'static' });
}

export default function S3Storages() {
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getS3StoragesOptions());
  const deleteStorage = useDeleteS3Storage();

  const [pendingDelete, setPendingDelete] = useState<S3Storage | null>(null);

  const handleConfirmDelete = async () => {
    if (!pendingDelete) {
      return;
    }

    const promise = deleteStorage.mutateAsync(pendingDelete.id);

    toast.promise(promise, {
      loading: 'Deleting S3 storage...',
      success: 'S3 storage deleted successfully',
      error: (err) => err.message || 'Failed to delete S3 storage.',
    });

    try {
      await promise;
      setPendingDelete(null);
    } catch {
      // Handled by toast
    }
  };

  return (
    <Page className="space-y-6">
      <TabActions>
        <Button
          label="Add storage"
          size="sm"
          icon={<PlusIcon className="size-3.5" />}
          onClick={() => navigate('/settings/s3-storages/new')}
        />
      </TabActions>

      <p className="max-w-prose text-pretty text-sm text-text-secondary">
        External S3-compatible storage buckets for offsite database backups,
        disaster recovery, and snapshot persistence.
      </p>

      <S3StorageList
        storages={data.storages ?? []}
        onDelete={setPendingDelete}
        onAddFirst={() => navigate('/settings/s3-storages/new')}
      />

      <ConfirmationDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title="Delete S3 Storage"
        description={`Are you sure you want to delete "${
          pendingDelete?.name ?? ''
        }"? Services using this storage will no longer be able to push backups offsite.`}
        confirmLabel="Delete"
        isPending={deleteStorage.isPending}
        onConfirm={handleConfirmDelete}
      />
    </Page>
  );
}
