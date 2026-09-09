import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Page } from '~/components/global/page';
import { PageHeader } from '~/components/interface/page-header';
import { SshKeyForm } from '~/components/settings/ssh-key-form';
import { useCreateSshKey } from '~/queries/ssh-keys';

export function meta() {
  return [{ title: 'Add SSH key · slasha' }];
}

export default function NewSshKey() {
  const navigate = useNavigate();
  const createKey = useCreateSshKey();

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const name = (formData.get('name') as string).trim();
    const public_key = (formData.get('public_key') as string).trim();

    const promise = createKey.mutateAsync({ name, public_key });

    toast.promise(promise, {
      loading: 'Adding SSH key...',
      success: () => {
        navigate('/settings/ssh-keys');
        return 'SSH key added successfully';
      },
      error: (err) => err.message || 'Failed to add SSH key.',
    });
  };

  return (
    <Page className="max-w-xl">
      <PageHeader
        title="Add SSH key"
        description="Paste a public key to grant it access to your applications over Git."
      />

      <div className="mt-6">
        <SshKeyForm
          onSubmit={handleSubmit}
          onCancel={() => navigate('/settings/ssh-keys')}
          isPending={createKey.isPending}
          submitLabel="Add key"
        />
      </div>
    </Page>
  );
}
