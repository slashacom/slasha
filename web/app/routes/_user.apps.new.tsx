import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { useCreateApp } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'New app · slasha' }];
}

export default function NewApp() {
  const navigate = useNavigate();
  const createApp = useCreateApp();

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const name = formData.get('name') as string;

    const promise = createApp.mutateAsync({ name });

    toast.promise(promise, {
      loading: 'Creating application...',
      success: `Successfully created ${name}`,
      error: (err) => err.message || 'Failed to create application.',
    });

    try {
      const data = await promise;
      await queryClient.invalidateQueries({ queryKey: ['apps'] });
      navigate(`/apps/${data.app.slug}`);
    } catch {}
  };

  return (
    <div>
      <div>
        <h3 className="font-semibold text-text">New app</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Give your application a name. We'll set up a git repository for you.
        </p>
      </div>

      <div className="mt-6">
        <form onSubmit={handleSubmit} className="w-full max-w-md">
          <div className="space-y-5">
            <div className="space-y-1.5">
              <Label
                htmlFor="name"
                className="text-[13px] font-medium text-text-secondary"
              >
                Application name
              </Label>
              <Input
                id="name"
                name="name"
                type="text"
                required
                placeholder="my-awesome-app"
                autoFocus
                className="h-10"
              />
              <p className="text-xs text-text-tertiary">
                Used to generate the slug and git repository name.
              </p>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2">
              <Button
                variant="ghost"
                label="Cancel"
                onClick={() => navigate('/apps')}
                isDisabled={createApp.isPending}
              />
              <Button
                type="submit"
                label="Create app"
                isLoading={createApp.isPending}
                isDisabled={createApp.isPending}
              />
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}
