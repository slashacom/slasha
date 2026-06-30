import { useQueryClient } from '@tanstack/react-query';
import { GitBranch } from 'lucide-react';
import { toast } from 'sonner';
import { useUpdateAppSettings } from '~/queries/apps';
import type { App } from '~/models/app';
import { Switch } from '~/components/interface/switch';
import { HStack, VStack } from '~/components/interface/stacks';

type AutoDeployManagerProps = {
  app: App;
};

export function AutoDeployManager(props: AutoDeployManagerProps) {
  const { app } = props;
  const queryClient = useQueryClient();
  const updateSettings = useUpdateAppSettings();

  const handleToggle = async (checked: boolean) => {
    try {
      await updateSettings.mutateAsync({
        appSlug: app.slug,
        auto_deploy: checked,
      });
      toast.success(checked ? 'Auto-deploy enabled' : 'Auto-deploy disabled');
      queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to update auto-deploy setting');
    }
  };

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div className="px-6 py-5">
          <HStack justifyContent="between" alignItems="center">
            <HStack space={3}>
              <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
                <GitBranch className="size-5" />
              </div>
              <div>
                <h3 className="text-[15px] font-semibold text-text">
                  Git Auto-Deploy
                </h3>
                <p className="mt-0.5 text-[13px] text-text-tertiary">
                  Automatically trigger a new deployment whenever a commit is
                  pushed to the default branch (
                  <span className="font-mono text-text-secondary">
                    {app.default_branch}
                  </span>
                  ).
                </p>
              </div>
            </HStack>
            <Switch
              checked={app.auto_deploy}
              onCheckedChange={handleToggle}
              disabled={updateSettings.isPending}
            />
          </HStack>
        </div>
      </div>
    </VStack>
  );
}
