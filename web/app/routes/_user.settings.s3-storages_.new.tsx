import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Page } from '~/components/global/page';
import { PageHeader } from '~/components/interface/page-header';
import {
  S3StorageForm,
  type S3StorageFormData,
} from '~/components/settings/s3-storage-form';
import { useCreateS3Storage } from '~/queries/s3-storage';

export function meta() {
  return [{ title: 'Add S3 storage · slasha' }];
}

export default function NewS3Storage() {
  const navigate = useNavigate();
  const createStorage = useCreateS3Storage();

  const handleSubmit = (data: S3StorageFormData) => {
    const promise = createStorage.mutateAsync({
      name: data.name,
      endpoint: data.endpoint,
      bucket: data.bucket,
      region: data.region,
      access_key_id: data.access_key_id,
      secret_access_key: data.secret_access_key || '',
      force_path_style: data.force_path_style,
    });

    toast.promise(promise, {
      loading: 'Adding S3 storage...',
      success: () => {
        navigate('/settings/s3-storages');
        return 'S3 storage added successfully';
      },
      error: (err) => err.message || 'Failed to add S3 storage.',
    });
  };

  return (
    <Page className="max-w-xl">
      <PageHeader
        title="Add S3 storage"
        description="Connect an S3 bucket or compatible object store (AWS, Cloudflare R2, MinIO, Wasabi) for database snapshots."
      />

      <div className="mt-6">
        <S3StorageForm
          onSubmit={handleSubmit}
          onCancel={() => navigate('/settings/s3-storages')}
          isPending={createStorage.isPending}
          submitLabel="Add storage"
        />
      </div>
    </Page>
  );
}
