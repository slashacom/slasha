import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Settings2 } from 'lucide-react';
import { toast } from 'sonner';
import { useUpdateAppSettings } from '~/queries/apps';
import type { App } from '~/models/app';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { HStack, VStack } from '~/components/interface/stacks';

type AppNameManagerProps = {
  app: App;
};

export function AppNameManager(props: AppNameManagerProps) {
  const { app } = props;
  const queryClient = useQueryClient();
  const updateSettings = useUpdateAppSettings();
  const [name, setName] = useState(app.name);

  const handleSave = async () => {
    const trimmed = name.trim();
    if (!trimmed || trimmed === app.name) {
      return;
    }

    const promise = updateSettings.mutateAsync({
      appSlug: app.slug,
      name: trimmed,
    });

    toast.promise(promise, {
      loading: 'Updating app name...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
        queryClient.invalidateQueries({ queryKey: ['apps'] });
        return 'App name updated successfully';
      },
      error: (error) => error.message || 'Failed to update app name.',
    });
  };

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div className="px-6 py-5">
          <HStack justifyContent="between" alignItems="start">
            <HStack space={3}>
              <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
                <Settings2 className="size-5" />
              </div>
              <div>
                <h3 className="text-[15px] font-semibold text-text">
                  Display Name
                </h3>
                <p className="mt-0.5 text-[13px] text-text-tertiary">
                  Change the display name of your application. The app slug and
                  URL will remain unchanged.
                </p>
                <div className="mt-4 flex items-center gap-3">
                  <Input
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                    placeholder="App Name"
                    className="w-64"
                  />
                  <Button
                    label="Save"
                    size="sm"
                    onClick={handleSave}
                    disabled={
                      updateSettings.isPending ||
                      name.trim() === app.name ||
                      !name.trim()
                    }
                  />
                </div>
              </div>
            </HStack>
          </HStack>
        </div>
      </div>
    </VStack>
  );
}
