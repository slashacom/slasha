import { useQueryClient } from '@tanstack/react-query';
import { GitBranch } from 'lucide-react';
import { toast } from 'sonner';
import { useUpdateAppSettings } from '~/queries/apps';
import type { App } from '~/models/app';
import { Switch } from '~/components/interface/switch';
import { SettingsCard } from '~/components/interface/settings-card';

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
    <SettingsCard
      icon={GitBranch}
      title="Git Auto-Deploy"
      description={
        <>
          Automatically trigger a new deployment whenever a commit is pushed to
          the default branch (
          <span className="font-mono text-text-secondary">
            {app.default_branch}
          </span>
          ).
        </>
      }
      actions={
        <Switch
          checked={app.auto_deploy}
          onCheckedChange={handleToggle}
          disabled={updateSettings.isPending}
        />
      }
    />
  );
}
