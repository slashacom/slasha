import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Settings2 } from 'lucide-react';
import { toast } from 'sonner';
import { useUpdateAppSettings } from '~/queries/apps';
import type { App } from '~/models/app';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { HStack } from '~/components/interface/stacks';
import { SettingsCard } from '~/components/interface/settings-card';

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
    <SettingsCard
      icon={Settings2}
      title="Display Name"
      description="Change the display name of your application. The app slug and URL will remain unchanged."
    >
      <HStack space={3}>
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
            updateSettings.isPending || name.trim() === app.name || !name.trim()
          }
        />
      </HStack>
    </SettingsCard>
  );
}
