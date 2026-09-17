import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Page } from '~/components/global/page';
import { PageHeader } from '~/components/interface/page-header';
import {
  S3StorageForm,
  type S3StorageFormData,
} from '~/components/settings/s3-storage-form';
import { getS3StorageOptions, useUpdateS3Storage } from '~/queries/s3-storage';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'Edit S3 storage · slasha' }];
}

export async function clientLoader(args: { params: { id: string } }) {
  await queryClient.query({
    ...getS3StorageOptions(args.params.id),
    staleTime: 'static',
  });
}

export default function EditS3Storage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getS3StorageOptions(id!));
  const updateStorage = useUpdateS3Storage();

  const handleSubmit = (formData: S3StorageFormData) => {
    const promise = updateStorage.mutateAsync({
      id: id!,
      payload: {
        name: formData.name,
        endpoint: formData.endpoint,
        bucket: formData.bucket,
        region: formData.region,
        access_key_id: formData.access_key_id,
        secret_access_key: formData.secret_access_key || undefined,
        force_path_style: formData.force_path_style,
      },
    });

    toast.promise(promise, {
      loading: 'Updating S3 storage...',
      success: () => {
        navigate('/settings/s3-storages');
        return 'S3 storage updated successfully';
      },
      error: (err) => err.message || 'Failed to update S3 storage.',
    });
  };

  return (
    <Page className="max-w-xl">
      <PageHeader
        title={`Edit ${data.storage.name}`}
        description="Update S3 endpoint, credentials, or addressing style."
      />

      <div className="mt-6">
        <S3StorageForm
          storage={data.storage}
          onSubmit={handleSubmit}
          onCancel={() => navigate('/settings/s3-storages')}
          isPending={updateStorage.isPending}
          submitLabel="Save changes"
        />
      </div>
    </Page>
  );
}
