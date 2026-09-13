import { Cloud } from 'lucide-react';
import type { S3Storage } from '~/models/s3-storage';
import { S3StorageRow } from './s3-storage-row';
import { EmptyPage } from '~/components/global/empty-page';
import { Table } from '~/components/interface/table';

type S3StorageListProps = {
  storages: S3Storage[];
  onDelete: (storage: S3Storage) => void;
  onAddFirst: () => void;
};

export function S3StorageList(props: S3StorageListProps) {
  const { storages, onDelete, onAddFirst } = props;

  return (
    <div className="mt-2 min-w-0 flex-1 overflow-x-auto">
      {storages.length === 0 ? (
        <EmptyPage
          icon={Cloud}
          title="No S3 storages configured."
          subtitle="Add an S3-compatible storage bucket (AWS S3, Cloudflare R2, MinIO, Wasabi) to store automated database backups."
          actionLabel="Add your first storage"
          onAction={onAddFirst}
        />
      ) : (
        <Table
          columns={[
            'Name / Bucket',
            'Endpoint',
            'Region',
            'Addressing',
            'Created',
            { label: '', align: 'right' },
          ]}
        >
          {storages.map((storage) => (
            <S3StorageRow
              key={storage.id}
              storage={storage}
              onDelete={onDelete}
            />
          ))}
        </Table>
      )}
    </div>
  );
}
