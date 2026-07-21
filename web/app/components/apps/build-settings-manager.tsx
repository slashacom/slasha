import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { FolderTree } from 'lucide-react';
import { toast } from 'sonner';
import { useUpdateAppSettings } from '~/queries/apps';
import type { App } from '~/models/app';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { HStack, VStack } from '~/components/interface/stacks';

type BuildSettingsManagerProps = {
  app: App;
};

export function BuildSettingsManager(props: BuildSettingsManagerProps) {
  const { app } = props;
  const queryClient = useQueryClient();
  const updateSettings = useUpdateAppSettings();
  const [rootDir, setRootDir] = useState(app.root_dir);

  const trimmed = rootDir.trim();

  const handleSave = async () => {
    if (trimmed === app.root_dir) {
      return;
    }

    const promise = updateSettings.mutateAsync({
      appSlug: app.slug,
      root_dir: trimmed,
    });

    toast.promise(promise, {
      loading: 'Updating root directory...',
      success: () => {
        queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
        return 'Root directory updated successfully';
      },
      error: (error) => error.message || 'Failed to update root directory.',
    });
  };

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div className="px-6 py-5">
          <HStack justifyContent="between" alignItems="start">
            <HStack space={3}>
              <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
                <FolderTree className="size-5" />
              </div>
              <div>
                <h3 className="text-[15px] font-semibold text-text">
                  Root Directory
                </h3>
                <p className="mt-0.5 text-[13px] text-text-tertiary">
                  Build from a subdirectory of the repository. Leave empty to
                  build from the repository root. Takes effect on the next
                  deployment.
                </p>
                <div className="mt-4 flex items-center gap-3">
                  <Input
                    value={rootDir}
                    onChange={(event) => setRootDir(event.target.value)}
                    placeholder="apps/web"
                    className="w-64 font-mono"
                  />
                  <Button
                    label="Save"
                    size="sm"
                    onClick={handleSave}
                    disabled={
                      updateSettings.isPending || trimmed === app.root_dir
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
