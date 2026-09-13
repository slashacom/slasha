import { useState, useEffect } from 'react';
import { useSuspenseQuery } from '@tanstack/react-query';
import {
  AlertCircle,
  AlertTriangle,
  Archive,
  CheckCircle2,
  CircleDashed,
  Cloud,
  CloudOff,
  Download,
  HardDrive,
  Play,
  RotateCcw,
  Settings2,
  Trash2,
  XCircle,
} from 'lucide-react';
import { toast } from 'sonner';
import { EmptyPage } from '~/components/global/empty-page';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { CopyButton } from '~/components/interface/copy-button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import { SettingsCard } from '~/components/interface/settings-card';
import { HStack } from '~/components/interface/stacks';
import { Table } from '~/components/interface/table';
import { TableRowActions } from '~/components/interface/table-row-actions';
import { cn } from '~/utils/classname';
import type { Service } from '~/models/service';
import type {
  ServiceBackup,
  ServiceBackupStatus,
  ServiceRestoreStatus,
} from '~/models/service-backup';
import { getS3StoragesOptions } from '~/queries/s3-storage';
import {
  getServiceBackupConfigOptions,
  getServiceBackupsOptions,
  useDeleteServiceBackup,
  useRestoreServiceBackup,
  useTriggerServiceBackup,
} from '~/queries/service-backups';
import type { ServiceRuntimeStatus } from '~/queries/services';
import { describeSchedule } from '~/utils/cron';
import { formatDateTime, formatRelativeTime } from '~/utils/date';
import { formatFileSize } from '~/utils/format';
import { getAuthToken } from '~/utils/jwt';
import { ServiceBackupConfigModal } from './service-backup-config-modal';

type ServiceBackupManagerProps = {
  appSlug: string;
  service: Service;
  runtimeStatus?: ServiceRuntimeStatus;
};

function BackupStatusBadge({
  status,
  restoreStatus,
  hasError,
  isRestoring,
  onViewError,
}: {
  status: ServiceBackupStatus;
  restoreStatus?: ServiceRestoreStatus;
  hasError?: boolean;
  isRestoring?: boolean;
  onViewError?: () => void;
}) {
  if (isRestoring || restoreStatus === 'restoring') {
    return (
      <span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded bg-amber-400/10 px-2 py-0.5 text-[11px] font-medium text-amber-400">
        <CircleDashed className="size-3 animate-spin" />
        Restoring
      </span>
    );
  }

  if (restoreStatus === 'failed') {
    return (
      <button
        type="button"
        onClick={onViewError}
        title="Restore failed. Click to view error details."
        className="inline-flex cursor-pointer items-center gap-1.5 whitespace-nowrap rounded bg-red-400/10 px-2 py-0.5 text-[11px] font-medium text-red-400 transition-opacity hover:opacity-80"
      >
        <XCircle className="size-3" />
        Restore Failed
      </button>
    );
  }

  if (status === 'running') {
    return (
      <span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded bg-sky-400/10 px-2 py-0.5 text-[11px] font-medium text-sky-400">
        <CircleDashed className="size-3 animate-spin" />
        Running
      </span>
    );
  }

  if (status === 'succeeded') {
    if (hasError) {
      return (
        <button
          type="button"
          onClick={onViewError}
          title="Local snapshot saved, but S3 replication failed. Click to view error."
          className="inline-flex cursor-pointer items-center gap-1.5 whitespace-nowrap rounded bg-amber-400/10 px-2 py-0.5 text-[11px] font-medium text-amber-400 transition-opacity hover:opacity-80"
        >
          <AlertTriangle className="size-3 shrink-0" />
          S3 Failed
        </button>
      );
    }

    return (
      <span className="inline-flex items-center gap-1.5 whitespace-nowrap rounded bg-emerald-400/10 px-2 py-0.5 text-[11px] font-medium text-emerald-400">
        <CheckCircle2 className="size-3" />
        Succeeded
      </span>
    );
  }

  return (
    <button
      type="button"
      onClick={onViewError}
      title="Backup failed. Click to view error."
      className="inline-flex cursor-pointer items-center gap-1.5 whitespace-nowrap rounded bg-red-400/10 px-2 py-0.5 text-[11px] font-medium text-red-400 transition-opacity hover:opacity-80"
    >
      <XCircle className="size-3" />
      Failed
    </button>
  );
}

function ServiceBackupContent(props: ServiceBackupManagerProps) {
  const { appSlug, service, runtimeStatus } = props;

  const { data: configData } = useSuspenseQuery(
    getServiceBackupConfigOptions(appSlug, service.id)
  );
  const { data: storagesData } = useSuspenseQuery(getS3StoragesOptions());
  const { data: backupsData } = useSuspenseQuery({
    ...getServiceBackupsOptions(appSlug, service.id),
    refetchInterval: (query) => {
      const list = query.state.data?.backups ?? [];
      const hasActive = list.some(
        (b) => b.status === 'running' || b.restore_status === 'restoring'
      );
      return hasActive ||
        runtimeStatus === 'restoring' ||
        runtimeStatus === 'backing up'
        ? 2500
        : 15000;
    },
  });

  const triggerBackup = useTriggerServiceBackup(appSlug, service.id);
  const restoreBackup = useRestoreServiceBackup(appSlug, service.id);
  const deleteBackup = useDeleteServiceBackup(appSlug, service.id);

  const config = configData?.config;
  const storages = storagesData?.storages ?? [];
  const backups = backupsData?.backups ?? [];

  const [showConfigModal, setShowConfigModal] = useState(false);
  const [restoreTarget, setRestoreTarget] = useState<ServiceBackup | null>(
    null
  );
  const [deleteTarget, setDeleteTarget] = useState<ServiceBackup | null>(null);
  const [restoringId, setRestoringId] = useState<string | null>(null);
  const [errorModal, setErrorModal] = useState<{
    title: string;
    fileName: string;
    error: string;
  } | null>(null);

  const isRunning =
    runtimeStatus !== undefined
      ? runtimeStatus === 'running'
      : service.status.toLowerCase() === 'running';
  const isAnyRestoring =
    restoringId !== null ||
    runtimeStatus === 'restoring' ||
    backups.some((b) => b.restore_status === 'restoring');
  const isAnyBackingUp =
    triggerBackup.isPending ||
    runtimeStatus === 'backing up' ||
    backups.some((b) => b.status === 'running');

  useEffect(() => {
    if (restoringId) {
      const target = backups.find((b) => b.id === restoringId);
      if (target && target.restore_status !== 'restoring') {
        setRestoringId(null);
      }
    }
  }, [backups, restoringId]);

  useEffect(() => {
    if (
      runtimeStatus &&
      runtimeStatus !== 'restoring' &&
      !backups.some((b) => b.restore_status === 'restoring')
    ) {
      setRestoringId(null);
    }
  }, [runtimeStatus, backups]);

  const handleTriggerBackup = async () => {
    const promise = triggerBackup.mutateAsync();
    toast.promise(promise, {
      loading: 'Starting database snapshot...',
      success: 'Database backup initiated successfully',
      error: (err) => err.message || 'Failed to trigger backup.',
    });
  };

  const handleConfirmRestore = async () => {
    if (!restoreTarget) return;

    const targetId = restoreTarget.id;
    setRestoringId(targetId);

    const promise = restoreBackup.mutateAsync(targetId);
    toast.promise(promise, {
      loading: `Restoring database from snapshot "${restoreTarget.file_name}"...`,
      success:
        'Database restore initiated successfully. Container will restart.',
      error: (err) => err.message || 'Failed to restore database backup.',
    });

    try {
      await promise;
      setRestoreTarget(null);
    } catch {
      setRestoringId(null);
    }
  };

  const handleConfirmDelete = async () => {
    if (!deleteTarget) return;

    const promise = deleteBackup.mutateAsync(deleteTarget.id);
    toast.promise(promise, {
      loading: 'Deleting backup snapshot...',
      success: 'Backup deleted successfully',
      error: (err) => err.message || 'Failed to delete backup.',
    });

    try {
      await promise;
      setDeleteTarget(null);
    } catch {}
  };

  const handleDownloadBackup = (backup: ServiceBackup) => {
    const token = getAuthToken();
    const query = token ? `?token=${encodeURIComponent(token)}` : '';
    const downloadUrl = `/api/apps/${appSlug}/services/${service.id}/backups/${backup.id}/download${query}`;
    const link = document.createElement('a');
    link.href = downloadUrl;
    link.download = backup.file_name;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const s3StorageObj = storages.find((s) => s.id === config?.s3_storage_id);
  const scheduleDescription = config?.schedule
    ? describeSchedule(config.schedule)
    : null;

  const storageDestinationLabel = config
    ? config.keep_local && s3StorageObj
      ? `local + ${s3StorageObj.name}`
      : config.keep_local
        ? 'local host'
        : s3StorageObj
          ? s3StorageObj.name
          : 'None'
    : '';

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-5">
      {/* Automated Backups Configuration Card */}
      <SettingsCard
        icon={Archive}
        title="Automated Backups"
        description={
          <span className="flex flex-col gap-1">
            <span className="flex items-center gap-2">
              {config?.enabled ? (
                <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-400/10 px-2 py-0.5 text-[11px] font-medium text-emerald-400">
                  <span className="size-1.5 rounded-full bg-emerald-400 animate-pulse" />
                  Active
                </span>
              ) : (
                <span className="rounded-full bg-white/5 px-2 py-0.5 text-[11px] font-medium text-text-tertiary">
                  Disabled
                </span>
              )}
              <span className="text-[13px] text-text-tertiary">
                {config?.enabled
                  ? `${scheduleDescription || config.schedule} (${config.timezone})${
                      config.next_run_at
                        ? ` · Next: ${formatDateTime(config.next_run_at)}`
                        : ''
                    } · Retaining ${config.retention_count} · Storage: ${storageDestinationLabel}`
                  : 'Automated backups are currently disabled. Schedule regular database snapshots and offsite replication.'}
              </span>
            </span>
          </span>
        }
        actions={
          <HStack space={2}>
            <Button
              label="Configure"
              variant="default"
              color="neutral"
              size="sm"
              icon={<Settings2 className="size-3.5" />}
              onClick={() => setShowConfigModal(true)}
            />

            <Button
              label={isAnyBackingUp ? 'Backing up...' : 'Backup now'}
              size="sm"
              icon={<Play className="size-3" />}
              isLoading={isAnyBackingUp}
              isDisabled={isAnyBackingUp || isAnyRestoring || !isRunning}
              onClick={handleTriggerBackup}
            />
          </HStack>
        }
      />

      {/* Snapshots Section: Empty State or Data Table */}
      {backups.length === 0 ? (
        <EmptyPage
          icon={HardDrive}
          size="lg"
          title="No database snapshots yet."
          subtitle={
            !isRunning
              ? 'Start the service container to capture snapshots or configure an automated backup schedule.'
              : 'Trigger a manual snapshot now or configure an automated schedule above to protect your database.'
          }
          actionLabel={isRunning ? 'Backup now' : undefined}
          actionIcon={<Play className="size-3.5" />}
          onAction={handleTriggerBackup}
          secondaryLabel="Configure schedule"
          onSecondaryAction={() => setShowConfigModal(true)}
          className="my-1 flex-1"
        />
      ) : (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <HardDrive className="size-4 text-text-secondary" />
              <h3 className="text-xs font-semibold text-text">Snapshots</h3>
              <span className="rounded-full bg-white/5 px-2 py-0.5 text-[11px] font-medium text-text-tertiary">
                {backups.length}
              </span>
            </div>
          </div>

          <div className="-mx-8 overflow-x-auto">
            <Table
              columns={[
                'Status',
                'Snapshot File',
                'Size',
                'Storage',
                'Trigger',
                'Created',
                { label: '', align: 'right' },
              ]}
            >
              {backups.map((backup) => {
                const storageObj = storages.find(
                  (s) => s.id === backup.s3_storage_id
                );
                const hasS3Error =
                  backup.status === 'succeeded' && Boolean(backup.error);
                const isBackupFailed = backup.status === 'failed';

                return (
                  <tr
                    key={backup.id}
                    className="group transition-colors hover:bg-white/[0.01]"
                  >
                    <td className="py-3.5 pr-4 align-middle">
                      <BackupStatusBadge
                        status={backup.status}
                        restoreStatus={backup.restore_status}
                        hasError={hasS3Error}
                        isRestoring={restoringId === backup.id}
                        onViewError={
                          backup.restore_status === 'failed' &&
                          backup.restore_error
                            ? () =>
                                setErrorModal({
                                  title: 'Restore Error',
                                  fileName: backup.file_name,
                                  error:
                                    backup.restore_error ||
                                    'Unknown restore error',
                                })
                            : hasS3Error
                              ? () =>
                                  setErrorModal({
                                    title: 'S3 Replication Error',
                                    fileName: backup.file_name,
                                    error: backup.error || 'Unknown error',
                                  })
                              : isBackupFailed
                                ? () =>
                                    setErrorModal({
                                      title: 'Backup Error',
                                      fileName: backup.file_name,
                                      error:
                                        backup.error || 'Unknown backup error',
                                    })
                                : undefined
                        }
                      />
                    </td>

                    <td className="py-3.5 pr-4 align-middle">
                      <div
                        className="max-w-[320px] truncate font-mono text-[12px] text-text"
                        title={backup.file_name}
                      >
                        {backup.file_name}
                      </div>

                      {backup.restore_status === 'failed' &&
                      backup.restore_error ? (
                        <button
                          type="button"
                          onClick={() =>
                            setErrorModal({
                              title: 'Restore Error',
                              fileName: backup.file_name,
                              error:
                                backup.restore_error || 'Unknown restore error',
                            })
                          }
                          className="mt-0.5 inline-flex cursor-pointer items-center gap-1 text-[11px] text-red-400 hover:text-red-300"
                        >
                          <AlertCircle className="size-2.5 shrink-0" />
                          <span>Restore failed · View error</span>
                        </button>
                      ) : null}

                      {backup.restore_status === 'succeeded' &&
                      backup.last_restored_at ? (
                        <div
                          className="mt-0.5 flex items-center gap-1 text-[11px] text-text-tertiary"
                          title={`Restored at ${formatDateTime(backup.last_restored_at)}`}
                        >
                          <RotateCcw className="size-2.5 shrink-0 text-emerald-400" />
                          <span>
                            Restored{' '}
                            {formatRelativeTime(backup.last_restored_at)}
                          </span>
                        </div>
                      ) : null}
                    </td>

                    <td className="py-3.5 pr-4 align-middle font-mono text-[12px] text-text-secondary">
                      {backup.file_size > 0
                        ? formatFileSize(Number(backup.file_size))
                        : '—'}
                    </td>

                    <td className="py-3.5 pr-4 align-middle text-[12px]">
                      {backup.stored_locally && storageObj ? (
                        hasS3Error ? (
                          <button
                            type="button"
                            onClick={() =>
                              setErrorModal({
                                title: 'S3 Replication Error',
                                fileName: backup.file_name,
                                error: backup.error || 'Unknown S3 error',
                              })
                            }
                            className="inline-flex cursor-pointer items-center gap-1.5 text-amber-400 hover:text-amber-300"
                            title="S3 upload failed. Click to view error."
                          >
                            <CloudOff className="size-3.5 shrink-0" />
                            <span>Local only (S3 failed)</span>
                          </button>
                        ) : (
                          <div className="flex items-center gap-1.5 text-text-secondary">
                            <Cloud className="size-3.5 text-sky-400 shrink-0" />
                            <span>Local + {storageObj.name}</span>
                          </div>
                        )
                      ) : storageObj ? (
                        hasS3Error ? (
                          <button
                            type="button"
                            onClick={() =>
                              setErrorModal({
                                title: 'S3 Replication Error',
                                fileName: backup.file_name,
                                error: backup.error || 'Unknown S3 error',
                              })
                            }
                            className="inline-flex cursor-pointer items-center gap-1.5 text-red-400 hover:text-red-300"
                            title="S3 upload failed. Click to view error."
                          >
                            <CloudOff className="size-3.5 shrink-0" />
                            <span>{storageObj.name} (failed)</span>
                          </button>
                        ) : (
                          <div className="flex items-center gap-1.5 text-text-secondary">
                            <Cloud className="size-3.5 text-sky-400 shrink-0" />
                            <span>{storageObj.name}</span>
                          </div>
                        )
                      ) : (
                        <div className="flex items-center gap-1.5 text-text-secondary">
                          <HardDrive className="size-3.5 text-emerald-400 shrink-0" />
                          <span>Local</span>
                        </div>
                      )}
                    </td>

                    <td className="py-3.5 pr-4 align-middle">
                      <span className="inline-flex items-center rounded bg-white/[0.03] px-2 py-0.5 text-[11px] text-text-tertiary">
                        {backup.trigger_kind === 'scheduled'
                          ? 'Scheduled'
                          : 'Manual'}
                      </span>
                    </td>

                    <td
                      className="py-3.5 pr-4 align-middle text-[12px] text-text-tertiary"
                      title={formatDateTime(backup.created_at)}
                    >
                      {formatRelativeTime(backup.created_at)}
                    </td>

                    <td className="py-3.5 text-right align-middle">
                      <div className="flex items-center justify-end gap-1">
                        {backup.status === 'succeeded' && (
                          <button
                            type="button"
                            onClick={() => handleDownloadBackup(backup)}
                            className="inline-flex size-7 cursor-pointer items-center justify-center rounded text-text-tertiary transition-colors hover:bg-white/5 hover:text-text"
                            title="Download snapshot"
                          >
                            <Download className="size-3.5" />
                          </button>
                        )}

                        <TableRowActions
                          actions={[
                            ...(backup.status === 'succeeded'
                              ? [
                                  {
                                    label: isAnyRestoring
                                      ? 'Restore in progress...'
                                      : isAnyBackingUp
                                        ? 'Backup in progress...'
                                        : !isRunning
                                          ? 'Service not running'
                                          : 'Restore database',
                                    icon: RotateCcw,
                                    isDestructive: true,
                                    isDisabled:
                                      isAnyRestoring ||
                                      isAnyBackingUp ||
                                      !isRunning,
                                    onClick: () => setRestoreTarget(backup),
                                  },
                                ]
                              : []),
                            {
                              label: 'Delete snapshot',
                              icon: Trash2,
                              isDestructive: true,
                              isDisabled:
                                isAnyRestoring &&
                                (restoringId === backup.id ||
                                  backup.restore_status === 'restoring'),
                              onClick: () => setDeleteTarget(backup),
                            },
                          ]}
                        />
                      </div>
                    </td>
                  </tr>
                );
              })}
            </Table>
          </div>
        </div>
      )}

      {/* Configuration Modal */}
      <ServiceBackupConfigModal
        appSlug={appSlug}
        service={service}
        open={showConfigModal}
        onOpenChange={setShowConfigModal}
      />

      {/* Confirmation Dialogs */}
      <ConfirmationDialog
        open={restoreTarget !== null}
        onOpenChange={(open) => !open && setRestoreTarget(null)}
        title="Restore Database from Snapshot"
        description={`Are you sure you want to restore "${
          restoreTarget?.file_name ?? ''
        }" into "${service.name}"? This operation is destructive and will overwrite existing database data with this snapshot and restart the service. It is recommended to stop or pause active application deployments prior to restoring to avoid connection locks.`}
        confirmLabel="Restore database"
        isPending={restoreBackup.isPending}
        onConfirm={handleConfirmRestore}
      />

      <ConfirmationDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        title="Delete Backup Snapshot"
        description={`Are you sure you want to delete "${
          deleteTarget?.file_name ?? ''
        }"? This cannot be undone.`}
        confirmLabel="Delete snapshot"
        isPending={deleteBackup.isPending}
        onConfirm={handleConfirmDelete}
      />

      {/* Error Details Modal */}
      <Dialog
        open={errorModal !== null}
        onOpenChange={(open) => {
          if (!open) setErrorModal(null);
        }}
      >
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>{errorModal?.title ?? 'Error Details'}</DialogTitle>
            <DialogDescription>
              Diagnostic failure details for database snapshot{' '}
              <span className="font-mono text-text">
                {errorModal?.fileName}
              </span>
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-2.5 sm:grid-cols-2">
            <div className="min-w-0 rounded-md border border-border bg-bg/40 p-2.5">
              <p className="text-[11px] font-medium text-text-tertiary">
                Snapshot
              </p>
              <p
                className="mt-0.5 truncate font-mono text-xs text-text select-all"
                title={errorModal?.fileName}
              >
                {errorModal?.fileName}
              </p>
            </div>
            <div className="min-w-0 rounded-md border border-border bg-bg/40 p-2.5">
              <p className="text-[11px] font-medium text-text-tertiary">
                Failure source
              </p>
              <p className="mt-0.5 text-xs font-medium text-red-400">
                {errorModal?.title ?? 'Database Error'}
              </p>
            </div>
          </div>

          <div className="overflow-hidden rounded-lg border border-border bg-code-bg">
            <div className="flex items-center justify-between border-b border-border bg-white/[0.02] px-3.5 py-2">
              <div className="flex items-center gap-2">
                <span className="size-2 rounded-full bg-red-400" />
                <span className="font-mono text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
                  Output / Stderr
                </span>
              </div>
              {errorModal ? (
                <CopyButton
                  value={errorModal.error}
                  label="Copy error message"
                />
              ) : null}
            </div>
            <pre className="custom-scrollbar max-h-72 overflow-x-auto overflow-y-auto p-3.5 font-mono text-[12px] leading-relaxed text-red-300/90 select-all whitespace-pre-wrap break-all">
              {errorModal?.error}
            </pre>
          </div>

          <DialogFooter className="mt-2">
            <Button
              label="Close"
              variant="ghost"
              onClick={() => setErrorModal(null)}
            />
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export function ServiceBackupManager(props: ServiceBackupManagerProps) {
  const { service } = props;

  if (service.kind === 'Redis') {
    return (
      <EmptyPage
        icon={Archive}
        size="lg"
        title="Backups not supported for Redis."
        subtitle="Redis operates as an in-memory datastore and cache. Automated snapshots and point-in-time restores are supported for PostgreSQL, MySQL, and MongoDB services."
        className="my-1 flex-1"
      />
    );
  }

  return <ServiceBackupContent {...props} />;
}
