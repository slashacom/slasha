import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { useQuery } from '@tanstack/react-query';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { getCheckSlugOptions, useCreateApp } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';
import { useDebounce } from '~/hooks/use-debounce';

export function meta() {
  return [{ title: 'New app · slasha' }];
}

export default function NewApp() {
  const navigate = useNavigate();
  const createApp = useCreateApp();
  const [name, setName] = useState('');
  const debouncedName = useDebounce(name, 300);

  const { data: slugCheck, isFetching } = useQuery({
    ...getCheckSlugOptions(debouncedName),
    enabled: debouncedName.trim().length > 0,
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const submittedName = formData.get('name') as string;

    const promise = createApp.mutateAsync({ name: submittedName });

    toast.promise(promise, {
      loading: 'Creating application...',
      success: `Successfully created ${submittedName}`,
      error: (err) =>
        err.response?.data?.error ||
        err.message ||
        'Failed to create application.',
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
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
              <div className="h-5">
                {name.trim() === '' ? (
                  <p className="text-xs text-text-tertiary">
                    Used to generate the slug and git repository name.
                  </p>
                ) : isFetching || debouncedName !== name ? (
                  <p className="text-xs text-text-tertiary animate-pulse">
                    Checking availability...
                  </p>
                ) : slugCheck ? (
                  <p className="text-xs text-text-tertiary">
                    Repository:{' '}
                    <span className="font-mono text-text-secondary">
                      {slugCheck.slug}.git
                    </span>
                    {!slugCheck.available && (
                      <span className="ml-2 text-amber-500/90">
                        (Name taken, using suggested unique name)
                      </span>
                    )}
                  </p>
                ) : null}
              </div>
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
                isDisabled={
                  createApp.isPending || debouncedName !== name || isFetching
                }
              />
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}
