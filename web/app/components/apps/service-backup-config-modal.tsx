import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import { Input } from '~/components/interface/input';
import { Select } from '~/components/interface/select';
import { HStack } from '~/components/interface/stacks';
import { Switch } from '~/components/interface/switch';
import type { Service } from '~/models/service';
import { getS3StoragesOptions } from '~/queries/s3-storage';
import {
  getServiceBackupConfigOptions,
  useUpdateServiceBackupConfig,
} from '~/queries/service-backups';
import { cn } from '~/utils/classname';
import { describeSchedule } from '~/utils/cron';
import { formatDateTime } from '~/utils/date';

type ServiceBackupConfigModalProps = {
  appSlug: string;
  service: Service;
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

const SCHEDULE_PRESETS: { label: string; value: string }[] = [
  { label: 'Every hour', value: '0 * * * *' },
  { label: 'Every 6 hours', value: '0 */6 * * *' },
  { label: 'Every 12 hours', value: '0 */12 * * *' },
  { label: 'Every day at midnight', value: '0 0 * * *' },
  { label: 'Every day at 3 AM', value: '0 3 * * *' },
  { label: 'Every Sunday at midnight', value: '0 0 * * 0' },
  { label: 'First of the month', value: '0 0 1 * *' },
];

const SUPPORTED_TIMEZONES = (
  Intl as typeof Intl & {
    supportedValuesOf?: (key: 'timeZone') => string[];
  }
).supportedValuesOf?.('timeZone') ?? ['UTC'];

const TIMEZONES = [
  'UTC',
  ...SUPPORTED_TIMEZONES.filter((zone) => zone !== 'UTC'),
];

export function ServiceBackupConfigModal(props: ServiceBackupConfigModalProps) {
  const { appSlug, service, open, onOpenChange } = props;

  const { data: configData } = useQuery(
    getServiceBackupConfigOptions(appSlug, service.id)
  );
  const { data: storagesData } = useQuery(getS3StoragesOptions());
  const updateConfig = useUpdateServiceBackupConfig(appSlug, service.id);

  const config = configData?.config;
  const storages = storagesData?.storages ?? [];

  const [enabled, setEnabled] = useState(config?.enabled ?? false);
  const [schedule, setSchedule] = useState(config?.schedule ?? '0 0 * * *');
  const [timezone, setTimezone] = useState(config?.timezone ?? 'UTC');
  const [retentionCount, setRetentionCount] = useState(
    String(config?.retention_count ?? 7)
  );
  const [s3StorageId, setS3StorageId] = useState<string>(
    config?.s3_storage_id ?? ''
  );
  const [keepLocal, setKeepLocal] = useState(config?.keep_local ?? true);

  useEffect(() => {
    if (config && open) {
      setEnabled(config.enabled);
      setSchedule(config.schedule);
      setTimezone(config.timezone);
      setRetentionCount(String(config.retention_count));
      setS3StorageId(config.s3_storage_id ?? '');
      setKeepLocal(config.keep_local ?? true);
    }
  }, [config, open]);

  if (service.kind === 'Redis') {
    return null;
  }

  const matchedPreset = SCHEDULE_PRESETS.find(
    (preset) => preset.value === schedule
  );
  const isCustomSchedule = !matchedPreset;
  const scheduleDescription = describeSchedule(schedule);

  const handleSave = async () => {
    const count = parseInt(retentionCount, 10);
    if (isNaN(count) || count < 1) {
      toast.error('Retention limit must be at least 1');
      return;
    }

    if (!keepLocal && !s3StorageId) {
      toast.error(
        'At least one storage destination (Local or S3) must be enabled'
      );
      return;
    }

    const promise = updateConfig.mutateAsync({
      enabled,
      schedule: schedule.trim(),
      timezone: timezone.trim() || 'UTC',
      retention_count: count,
      s3_storage_id: s3StorageId ? s3StorageId : null,
      keep_local: keepLocal,
    });

    toast.promise(promise, {
      loading: 'Saving backup configuration...',
      success: 'Backup configuration saved successfully',
      error: (err) => err.message || 'Failed to save backup configuration.',
    });

    try {
      await promise;
      onOpenChange(false);
    } catch {}
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Backup Configuration</DialogTitle>
          <DialogDescription>
            Configure storage destinations and optional automated snapshot
            schedules for{' '}
            <span className="font-mono text-text">{service.name}</span>.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-1">
          <div className="space-y-3 rounded-lg border border-border bg-surface/30 p-3.5">
            <span className="text-xs font-medium text-text-secondary">
              Storage Destinations
            </span>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <span className="text-[13px] font-medium text-text">
                  Local Host Storage
                </span>
                <p className="text-[11px] text-text-tertiary">
                  Keep snapshot on the server host for instant restoration.
                </p>
              </div>
              <Switch
                checked={keepLocal}
                onCheckedChange={setKeepLocal}
                aria-label="Local host storage"
              />
            </div>

            <div className="space-y-2 border-t border-border/50 pt-3">
              <label className="text-xs font-medium text-text-secondary">
                Offsite S3 Storage
              </label>
              <Select
                value={s3StorageId}
                onChange={(e) => setS3StorageId(e.target.value)}
              >
                <option value="">None (do not replicate to S3)</option>
                {storages.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name} ({s.bucket})
                  </option>
                ))}
              </Select>
              <p className="text-[11px] leading-relaxed text-text-tertiary">
                Upload snapshots to an external S3-compatible cloud bucket for
                disaster recovery.
              </p>
            </div>

            {!keepLocal && !s3StorageId && (
              <p className="text-[11px] font-medium text-red-400">
                Select an S3 bucket or enable Local Host Storage.
              </p>
            )}
          </div>

          <div className="flex items-center justify-between rounded-lg border border-border bg-surface/50 p-3.5">
            <div className="space-y-0.5">
              <span className="text-[13px] font-medium text-text">
                Enable Automated Backups
              </span>
              <p className="text-[11px] text-text-tertiary">
                Automatically capture snapshots on the configured schedule.
              </p>
            </div>
            <Switch
              checked={enabled}
              onCheckedChange={setEnabled}
              aria-label="Enable automated backups"
            />
          </div>

          <div
            className={cn(
              'space-y-4 transition-opacity',
              !enabled && 'opacity-60'
            )}
          >
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="text-xs font-medium text-text-secondary">
                  Backup Schedule
                </label>
                {enabled && config?.next_run_at ? (
                  <span className="text-[11px] text-text-tertiary">
                    Next run:{' '}
                    <span className="font-mono text-text-secondary">
                      {formatDateTime(config.next_run_at)}
                    </span>
                  </span>
                ) : null}
              </div>

              <Select
                value={isCustomSchedule ? 'custom' : schedule}
                onChange={(e) => {
                  if (e.target.value === 'custom') {
                    setSchedule('');
                  } else {
                    setSchedule(e.target.value);
                  }
                }}
                disabled={!enabled}
              >
                {SCHEDULE_PRESETS.map((preset) => (
                  <option key={preset.value} value={preset.value}>
                    {preset.label} ({preset.value})
                  </option>
                ))}
                <option value="custom">Custom cron expression...</option>
              </Select>

              {isCustomSchedule && (
                <Input
                  value={schedule}
                  onChange={(e) => setSchedule(e.target.value)}
                  placeholder="0 0 * * *"
                  className="font-mono text-xs"
                  disabled={!enabled}
                />
              )}

              <p className="text-[11px] text-text-tertiary">
                {scheduleDescription || 'Cron schedule expression'}
              </p>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-text-secondary">
                  Timezone
                </label>
                <Select
                  value={timezone}
                  onChange={(e) => setTimezone(e.target.value)}
                  disabled={!enabled}
                >
                  {TIMEZONES.map((tz) => (
                    <option key={tz} value={tz}>
                      {tz}
                    </option>
                  ))}
                </Select>
                <p className="text-[11px] text-text-tertiary">
                  Schedule evaluation zone
                </p>
              </div>

              <div className="space-y-1.5">
                <label className="text-xs font-medium text-text-secondary">
                  Retention Limit
                </label>
                <div className="relative">
                  <Input
                    type="number"
                    min="1"
                    max="100"
                    value={retentionCount}
                    onChange={(e) => setRetentionCount(e.target.value)}
                    disabled={!enabled}
                    className="pr-20 font-mono [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                  />
                  <span className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-xs text-text-tertiary">
                    snapshots
                  </span>
                </div>
                <p className="text-[11px] text-text-tertiary">
                  Oldest rotated automatically
                </p>
              </div>
            </div>
          </div>
        </div>

        <DialogFooter className="mt-6">
          <Button
            type="button"
            variant="ghost"
            label="Cancel"
            onClick={() => onOpenChange(false)}
          />
          <Button
            type="button"
            label="Save configuration"
            isLoading={updateConfig.isPending}
            isDisabled={updateConfig.isPending || (!keepLocal && !s3StorageId)}
            onClick={handleSave}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
