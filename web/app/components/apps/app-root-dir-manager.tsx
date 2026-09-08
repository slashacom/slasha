import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Folder } from 'lucide-react';
import { toast } from 'sonner';
import { useUpdateAppSettings, getAppDirectoriesOptions } from '~/queries/apps';
import type { App } from '~/models/app';
import { Button } from '~/components/interface/button';
import { Select } from '~/components/interface/select';
import { HStack } from '~/components/interface/stacks';
import { SettingsCard } from '~/components/interface/settings-card';

type AppRootDirManagerProps = {
  app: App;
};

export function AppRootDirManager(props: AppRootDirManagerProps) {
  const { app } = props;
  const queryClient = useQueryClient();
  const updateSettings = useUpdateAppSettings();
  const { data: dirsData, isLoading } = useQuery(
    getAppDirectoriesOptions(app.slug)
  );
  const [rootDir, setRootDir] = useState(app.root_dir || '');

  const handleSave = async () => {
    if (rootDir === (app.root_dir || '')) {
      return;
    }

    const promise = updateSettings.mutateAsync({
      appSlug: app.slug,
      root_dir: rootDir,
    });

    toast.promise(promise, {
      loading: 'Updating root directory...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
        queryClient.invalidateQueries({ queryKey: ['apps'] });
        return 'Root directory updated successfully';
      },
      error: (error) => error.message || 'Failed to update root directory.',
    });
  };

  const directories = dirsData?.directories || [];
  // ensure the root directory ('/' mapped to '') is always first and unique
  const allDirectories = [''].concat(directories.filter((d) => d !== ''));

  return (
    <SettingsCard
      icon={Folder}
      title="Root Directory"
      description="The directory within your repository where the application code resides."
    >
      <HStack space={3}>
        <Select
          value={rootDir}
          onChange={(event) => setRootDir(event.target.value)}
          disabled={isLoading}
          className="w-64"
        >
          {allDirectories.map((dir) => (
            <option key={dir} value={dir}>
              {dir === '' ? '/' : dir}
            </option>
          ))}
        </Select>
        <Button
          label="Save"
          size="sm"
          onClick={handleSave}
          disabled={
            updateSettings.isPending ||
            isLoading ||
            rootDir === (app.root_dir || '')
          }
        />
      </HStack>
    </SettingsCard>
  );
}
