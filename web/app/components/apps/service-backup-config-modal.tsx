import { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, Clock, HardDrive } from 'lucide-react';
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
          <div className="rounded-lg border border-border bg-surface/30 p-4 space-y-3.5">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-white/5 text-text-secondary">
                  <Clock className="size-4" />
                </div>
                <div>
                  <h4 className="text-[13px] font-semibold text-text">
                    Automated Backups
                  </h4>
                  <p className="text-[11px] text-text-tertiary">
                    Automatically capture and rotate snapshots on a schedule.
                  </p>
                </div>
              </div>
              <Switch
                checked={enabled}
                onCheckedChange={setEnabled}
                aria-label="Enable automated backups"
              />
            </div>

            {enabled ? (
              <div className="space-y-3.5 border-t border-border/50 pt-3.5">
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <label className="text-xs font-medium text-text-secondary">
                      Backup Schedule
                    </label>
                    {config?.next_run_at ? (
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
            ) : (
              <div className="border-t border-border/50 pt-3 text-[11px] text-text-tertiary">
                Automated backups are paused. Snapshots will only be created
                when triggered manually.
              </div>
            )}
          </div>

          <div className="rounded-lg border border-border bg-surface/30 p-4 space-y-3.5">
            <div className="flex items-center gap-2.5">
              <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-white/5 text-text-secondary">
                <HardDrive className="size-4" />
              </div>
              <div>
                <h4 className="text-[13px] font-semibold text-text">
                  Storage Destinations
                </h4>
                <p className="text-[11px] text-text-tertiary">
                  Where snapshots are saved when backups run.
                </p>
              </div>
            </div>

            <div className="space-y-3 border-t border-border/50 pt-3">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <span className="text-[13px] font-medium text-text">
                    Local Storage
                  </span>
                  <p className="text-[11px] text-text-tertiary">
                    Keep snapshots on the server's local storage for instant
                    restoration.
                  </p>
                </div>
                <Switch
                  checked={keepLocal}
                  onCheckedChange={setKeepLocal}
                  aria-label="Local storage"
                />
              </div>

              <div className="space-y-1.5 border-t border-border/50 pt-3">
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
                <div className="flex items-center gap-2 rounded-md border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] text-red-400">
                  <AlertCircle className="size-3.5 shrink-0" />
                  <span>
                    At least one destination (Local Storage or S3) must be
                    enabled.
                  </span>
                </div>
              )}
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
