import { useSuspenseQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack } from '~/components/interface/stacks';
import {
  getServerSettingsOptions,
  useUpdateServerSettings,
} from '~/queries/server-settings';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'Server Settings · slasha' }];
}

export async function clientLoader() {
  await queryClient.ensureQueryData(getServerSettingsOptions());
  return null;
}

export default function ServerSettingsPage() {
  const { data: settings } = useSuspenseQuery(getServerSettingsOptions());
  const updateSettings = useUpdateServerSettings();

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const form = e.currentTarget;
    const formData = new FormData(form);

    const cpuStr = formData.get('cpu_limit_percent') as string;
    const memoryStr = formData.get('memory_limit_percent') as string;
    const diskStr = formData.get('disk_limit_percent') as string;
    const webhook = formData.get('slack_webhook_url') as string;

    const payload = {
      id: settings.id,
      cpu_limit_percent: cpuStr ? parseFloat(cpuStr) : null,
      memory_limit_percent: memoryStr ? parseFloat(memoryStr) : null,
      disk_limit_percent: diskStr ? parseFloat(diskStr) : null,
      slack_webhook_url: webhook ? webhook : null,
      updated_at: settings.updated_at,
    };

    const promise = updateSettings.mutateAsync(payload);

    toast.promise(promise, {
      loading: 'Saving server settings...',
      success: 'Server settings updated successfully',
      error: (err) => err.message || 'Failed to update settings.',
    });
  };

  return (
    <div className="space-y-6 max-w-xl">
      <div>
        <h3 className="font-semibold text-text">Server Settings</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Configure server-level settings like alert thresholds and
          notifications.
        </p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6 max-w-md">
        <VStack space={4}>
          <VStack space={2}>
            <Label
              htmlFor="cpu_limit_percent"
              className="text-[13px] font-medium text-text-secondary"
            >
              CPU Limit (%)
            </Label>
            <Input
              id="cpu_limit_percent"
              name="cpu_limit_percent"
              type="number"
              step="0.1"
              min="0"
              max="100"
              key={`cpu-${settings.cpu_limit_percent}`}
              defaultValue={settings.cpu_limit_percent || ''}
              className="h-11 border-border bg-surface text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
              placeholder="e.g. 80"
            />
          </VStack>

          <VStack space={2}>
            <Label
              htmlFor="memory_limit_percent"
              className="text-[13px] font-medium text-text-secondary"
            >
              Memory Limit (%)
            </Label>
            <Input
              id="memory_limit_percent"
              name="memory_limit_percent"
              type="number"
              step="0.1"
              min="0"
              max="100"
              key={`mem-${settings.memory_limit_percent}`}
              defaultValue={settings.memory_limit_percent || ''}
              className="h-11 border-border bg-surface text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
              placeholder="e.g. 90"
            />
          </VStack>

          <VStack space={2}>
            <Label
              htmlFor="disk_limit_percent"
              className="text-[13px] font-medium text-text-secondary"
            >
              Disk Limit (%)
            </Label>
            <Input
              id="disk_limit_percent"
              name="disk_limit_percent"
              type="number"
              step="0.1"
              min="0"
              max="100"
              key={`disk-${settings.disk_limit_percent}`}
              defaultValue={settings.disk_limit_percent || ''}
              className="h-11 border-border bg-surface text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
              placeholder="e.g. 85"
            />
          </VStack>

          <hr className="border-border my-2" />

          <VStack space={2}>
            <Label
              htmlFor="slack_webhook_url"
              className="text-[13px] font-medium text-text-secondary"
            >
              Slack Webhook URL
            </Label>
            <Input
              id="slack_webhook_url"
              name="slack_webhook_url"
              type="url"
              key={`webhook-${settings.slack_webhook_url}`}
              defaultValue={settings.slack_webhook_url || ''}
              className="h-11 border-border bg-surface text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
              placeholder="https://hooks.slack.com/services/..."
            />
            <p className="text-[12px] text-text-tertiary">
              Alerts will be sent to this webhook when thresholds are exceeded.
            </p>
          </VStack>

          <div className="flex justify-start pt-4">
            <Button
              type="submit"
              isLoading={updateSettings.isPending}
              isDisabled={updateSettings.isPending}
              label="Save changes"
              className="h-11 px-6 justify-center bg-white text-bg hover:bg-white/90 focus:ring-0 focus:ring-offset-0 font-medium"
            />
          </div>
        </VStack>
      </form>
    </div>
  );
}
