import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { FormField } from '~/components/interface/form-field';
import { Input } from '~/components/interface/input';
import { Select } from '~/components/interface/select';
import { Switch } from '~/components/interface/switch';
import { Textarea } from '~/components/interface/textarea';
import { useDebounce } from '~/hooks/use-debounce';
import type { CronJob, CronRuntime } from '~/models/cron';
import {
  getCronPreviewOptions,
  useCreateCron,
  useUpdateCron,
} from '~/queries/crons';
import { CronSchedulePreview } from '~/components/apps/cron-schedule-preview';

type CronFormProps = {
  appSlug: string;
  cron?: CronJob;
  onCancel: () => void;
  onSaved: () => void;
};

export function CronForm(props: CronFormProps) {
  const { appSlug, cron, onCancel, onSaved } = props;
  const createCron = useCreateCron(appSlug);
  const updateCron = useUpdateCron(appSlug);
  const [name, setName] = useState(cron?.name ?? '');
  const [schedule, setSchedule] = useState(cron?.schedule ?? '0 * * * *');
  const [command, setCommand] = useState(cron?.command ?? '');
  const [timezone, setTimezone] = useState(cron?.timezone ?? 'UTC');
  const [timeoutSecs, setTimeoutSecs] = useState(
    String(cron?.timeout_secs ?? 3600)
  );
  const [runtime, setRuntime] = useState<CronRuntime>(cron?.runtime ?? 'app');
  const [enabled, setEnabled] = useState(cron?.enabled ?? true);

  const debouncedSchedule = useDebounce(schedule.trim(), 400);
  const debouncedTimezone = useDebounce(timezone.trim() || 'UTC', 400);
  const preview = useQuery({
    ...getCronPreviewOptions(appSlug, debouncedSchedule, debouncedTimezone),
    enabled: debouncedSchedule.length > 0,
  });

  const handleSave = async () => {
    if (!name.trim()) {
      toast.error('Name is required.');
      return;
    }
    if (!schedule.trim()) {
      toast.error('Schedule is required.');
      return;
    }
    if (!command.trim()) {
      toast.error('Command is required.');
      return;
    }

    const timeout = Number(timeoutSecs);
    if (!timeout || timeout <= 0) {
      toast.error('Timeout must be greater than zero.');
      return;
    }

    const payload = {
      name: name.trim(),
      schedule: schedule.trim(),
      command: command.trim(),
      timezone: timezone.trim() || 'UTC',
      enabled,
      timeout_secs: timeout,
      runtime,
    };
    const promise = cron
      ? updateCron.mutateAsync({ id: cron.id, data: payload })
      : createCron.mutateAsync(payload);

    toast.promise(promise, {
      loading: cron ? 'Updating job...' : 'Creating job...',
      success: cron ? 'Job updated.' : 'Job created.',
      error: (error) => error.message || 'Failed to save job.',
    });

    try {
      await promise;
      onSaved();
    } catch {
      return;
    }
  };

  return (
    <div className="max-w-2xl space-y-5">
      <FormField label="Name">
        <Input
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="Nightly cleanup"
        />
      </FormField>

      <FormField label="Schedule" help="cron expression">
        <Input
          value={schedule}
          onChange={(event) => setSchedule(event.target.value)}
          placeholder="0 3 * * *"
          className="font-mono"
        />
        <CronSchedulePreview
          loading={preview.isFetching}
          error={preview.isError ? (preview.error as Error).message : null}
          nextRuns={preview.data?.next_runs ?? []}
        />
      </FormField>

      <FormField label="Runtime" help="where the command runs">
        <Select
          value={runtime}
          onChange={(event) => setRuntime(event.target.value as CronRuntime)}
        >
          <option value="app">App image</option>
          <option value="utility">Utility (curl)</option>
        </Select>
      </FormField>

      <FormField label="Command">
        <Textarea
          value={command}
          onChange={(event) => setCommand(event.target.value)}
          placeholder={
            runtime === 'utility'
              ? `curl -sS -X POST -d '{"text":"hello"}' "$SLACK_WEBHOOK_URL"`
              : 'npm run cleanup'
          }
          className="font-mono"
        />
        <p className="mt-2 text-xs text-text-tertiary">
          {runtime === 'utility'
            ? 'Runs in a lightweight container with curl available. Your app’s environment variables are injected — good for webhooks and HTTP calls.'
            : 'Runs in your app’s container, with the same image, environment variables, and files. Available commands depend on what your image includes.'}
        </p>
      </FormField>

      <div className="grid gap-5 sm:grid-cols-2">
        <FormField label="Timezone" help="IANA, e.g. America/New_York">
          <Input
            value={timezone}
            onChange={(event) => setTimezone(event.target.value)}
            placeholder="UTC"
          />
        </FormField>
        <FormField label="Timeout (seconds)">
          <Input
            type="number"
            min={1}
            value={timeoutSecs}
            onChange={(event) => setTimeoutSecs(event.target.value)}
          />
        </FormField>
      </div>

      <div className="flex items-center justify-between border-t border-border pt-4">
        <div>
          <p className="text-sm font-medium text-text">Enabled</p>
          <p className="text-xs text-text-tertiary">
            Disabled jobs are saved but never run on their schedule.
          </p>
        </div>
        <Switch checked={enabled} onCheckedChange={setEnabled} />
      </div>

      <div className="flex items-center gap-2 pt-2">
        <Button
          label={cron ? 'Save changes' : 'Create job'}
          onClick={handleSave}
          isLoading={createCron.isPending || updateCron.isPending}
        />
        <Button label="Cancel" variant="ghost" onClick={onCancel} />
      </div>
    </div>
  );
}
