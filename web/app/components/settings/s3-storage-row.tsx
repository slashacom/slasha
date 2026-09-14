import { Cloud, Pencil, RefreshCw, Trash2Icon } from 'lucide-react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import type { S3Storage } from '~/models/s3-storage';
import { formatDate } from '~/utils/date';
import { TableRowActions } from '~/components/interface/table-row-actions';
import { useTestS3Storage } from '~/queries/s3-storage';
import { Button } from '~/components/interface/button';

type S3StorageRowProps = {
  storage: S3Storage;
  onDelete: (storage: S3Storage) => void;
};

export function S3StorageRow(props: S3StorageRowProps) {
  const { storage, onDelete } = props;
  const navigate = useNavigate();
  const testMutation = useTestS3Storage();

  const handleTestConnection = async () => {
    const promise = testMutation.mutateAsync(storage.id);

    toast.promise(promise, {
      loading: `Testing connection to "${storage.name}"...`,
      success: `Connected to bucket "${storage.bucket}" successfully!`,
      error: (err) => err.message || 'Failed to connect to S3 storage.',
    });

    try {
      await promise;
    } catch {
      // Handled by toast
    }
  };

  return (
    <tr className="group transition-colors hover:bg-white/[0.01]">
      <td className="py-4 pr-4 align-top">
        <div className="flex items-center gap-2.5">
          <Cloud className="size-4 text-text-tertiary" />
          <div className="min-w-0">
            <span className="block font-medium text-text">{storage.name}</span>
            <span className="block font-mono text-[11px] text-text-tertiary">
              {storage.bucket}
            </span>
          </div>
        </div>
      </td>
      <td className="py-4 pr-4 align-top">
        <code className="block max-w-[280px] truncate rounded bg-surface/50 px-1.5 py-0.5 font-mono text-[11px] text-text-secondary">
          {storage.endpoint}
        </code>
      </td>
      <td className="py-4 pr-4 align-top">
        <span className="inline-flex items-center rounded border border-white/5 bg-white/[0.03] px-2 py-0.5 font-mono text-[11px] text-text-secondary">
          {storage.region || 'auto'}
        </span>
      </td>
      <td className="py-4 pr-4 align-top">
        <span className="inline-flex items-center rounded px-2 py-0.5 text-[11px] font-medium text-text-tertiary">
          {storage.force_path_style ? 'Path-style' : 'Virtual-host'}
        </span>
      </td>
      <td className="py-4 pr-4 align-top text-[12px] text-text-tertiary">
        {formatDate(storage.created_at)}
      </td>
      <td className="py-4 text-right align-top">
        <div className="flex items-center justify-end gap-1">
          <Button
            label="Test"
            variant="ghost"
            size="sm"
            icon={<RefreshCw className="size-3.5" />}
            isLoading={testMutation.isPending}
            isDisabled={testMutation.isPending}
            onClick={handleTestConnection}
          />
          <TableRowActions
            actions={[
              {
                label: 'Edit storage',
                icon: Pencil,
                onClick: () =>
                  navigate(`/settings/s3-storages/${storage.id}/edit`),
              },
              {
                label: 'Delete storage',
                icon: Trash2Icon,
                isDestructive: true,
                onClick: () => onDelete(storage),
              },
            ]}
          />
        </div>
      </td>
    </tr>
  );
}
