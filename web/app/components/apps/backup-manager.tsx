import { useEffect, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  DatabaseBackup,
  History,
  Loader2,
  Save,
} from 'lucide-react';
import { toast } from 'sonner';

import {
  getBackupOptions,
  getBackupStatusOptions,
  getReplicaHealthOptions,
  getVolumesOptions,
  useRestoreBackup,
  useSaveBackup,
} from '~/queries/storage';
import { BackupStatusStrip } from '~/components/apps/backup-status-strip';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { FieldLabel } from '~/components/interface/field-label';
import { Input } from '~/components/interface/input';
import { Switch } from '~/components/interface/switch';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

const DEFAULT_DB_PATH = '/data/app.db';

type BackupManagerProps = {
  appSlug: string;
};

export function BackupManager(props: BackupManagerProps) {
  const { appSlug } = props;
  const queryClient = useQueryClient();
  const { data, isLoading } = useQuery(getBackupOptions(appSlug));
  const { data: volumesData } = useQuery(getVolumesOptions(appSlug));
  const saveBackup = useSaveBackup();
  const restoreBackup = useRestoreBackup();

  const [enabled, setEnabled] = useState(false);
  const [dbPath, setDbPath] = useState(DEFAULT_DB_PATH);
  const [bucket, setBucket] = useState('');
  const [endpoint, setEndpoint] = useState('');
  const [pathPrefix, setPathPrefix] = useState('');
  const [accessKeyId, setAccessKeyId] = useState('');
  const [secret, setSecret] = useState('');
  const [secretSet, setSecretSet] = useState(false);
  const [showRestoreConfirm, setShowRestoreConfirm] = useState(false);

  const backup = data?.backup ?? null;
  const savedEnabled = backup?.enabled ?? false;
  // Show the body (form/footer) when the toggle is on, or when an enabled backup
  // is being toggled off and still needs a save to persist.
  const hasBody = enabled || savedEnabled;

  const { data: statusData } = useQuery({
    ...getBackupStatusOptions(appSlug),
    refetchInterval: savedEnabled ? 10000 : false,
  });
  const status = statusData?.status;

  const healthProbe = useQuery({
    ...getReplicaHealthOptions(appSlug),
    enabled: savedEnabled,
    refetchInterval: savedEnabled ? 60000 : false,
    refetchOnWindowFocus: false,
  });
  const health = healthProbe.data;

  useEffect(() => {
    if (!backup) {
      return;
    }
    setEnabled(backup.enabled);
    setDbPath(backup.db_path || DEFAULT_DB_PATH);
    setBucket(backup.bucket);
    setEndpoint(backup.endpoint);
    setPathPrefix(backup.path_prefix ?? '');
    setAccessKeyId(backup.access_key_id);
    setSecretSet(backup.secret_set);
  }, [backup]);

  const volumes = volumesData?.volumes ?? [];
  const onPersistentVolume = volumes.some((volume) => {
    const prefix = volume.path.endsWith('/') ? volume.path : `${volume.path}/`;
    return dbPath === volume.path || dbPath.startsWith(prefix);
  });

  const handleSave = async () => {
    try {
      await saveBackup.mutateAsync({
        appSlug,
        enabled,
        db_path: dbPath.trim() || DEFAULT_DB_PATH,
        bucket: bucket.trim(),
        endpoint: endpoint.trim(),
        path_prefix: pathPrefix.trim() || undefined,
        access_key_id: accessKeyId.trim(),
        secret_access_key: secret || undefined,
      });
      setSecret('');
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'backups'] });
      toast.success('Backup settings saved. Redeploy to apply.');
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save backup settings');
    }
  };

  const handleRestore = async () => {
    try {
      await restoreBackup.mutateAsync(appSlug);
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'backups'] });
      toast.success(
        'Restore queued. Your database will be restored on the next deploy.'
      );
    } catch (e: any) {
      toast.error(e?.message || 'Failed to queue restore');
    }
  };

  if (isLoading) {
    return (
      <div className="flex h-32 items-center justify-center text-text-tertiary">
        <Loader2 className="size-5 animate-spin" />
      </div>
    );
  }

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div
          className={cn(
            'bg-surface/50 px-6 py-5',
            hasBody && 'border-b border-border'
          )}
        >
          <HStack justifyContent="between" alignItems="start">
            <HStack space={3}>
              <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
                <DatabaseBackup className="size-5" />
              </div>
              <div className="max-w-md">
                <h3 className="text-[15px] font-semibold text-text">
                  Litestream backups
                </h3>
                <p className="mt-0.5 text-[13px] text-text-tertiary">
                  Replicate a SQLite DB to object storage for offsite,
                  point-in-time recovery.
                </p>
              </div>
            </HStack>
            <Switch checked={enabled} onCheckedChange={setEnabled} />
          </HStack>
        </div>

        {enabled && savedEnabled && status ? (
          <BackupStatusStrip
            status={status}
            health={health}
            isChecking={healthProbe.isFetching}
            onCheck={() => healthProbe.refetch()}
          />
        ) : null}

        {enabled ? (
          <div className="space-y-5 p-6">
            <div>
              <FieldLabel label="Database path" />
              <Input
                value={dbPath}
                onChange={(e) => setDbPath(e.target.value)}
                placeholder={DEFAULT_DB_PATH}
                className="font-mono text-[13px]"
              />
              {onPersistentVolume ? (
                <p className="mt-1.5 text-[11px] leading-5 text-text-tertiary">
                  On a persistent volume — restarts are fast and the local copy
                  is kept.
                </p>
              ) : (
                <HStack
                  space={2}
                  alignItems="start"
                  className="mt-1.5 rounded-md border border-amber-500/20 bg-amber-500/5 px-2.5 py-2"
                >
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-amber-500" />
                  <p className="text-[11px] leading-5 text-amber-500/90">
                    This path isn't on a persistent volume, so it'll be restored
                    from object storage on every restart. Put it under{' '}
                    <span className="font-mono">/data</span> for fast, safe
                    restarts.
                  </p>
                </HStack>
              )}
            </div>

            <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
              <div>
                <FieldLabel label="Bucket" />
                <Input
                  value={bucket}
                  onChange={(e) => setBucket(e.target.value)}
                  placeholder="my-app-db"
                  className="text-[13px]"
                />
              </div>
              <div>
                <FieldLabel label="Path prefix (optional)" />
                <Input
                  value={pathPrefix}
                  onChange={(e) => setPathPrefix(e.target.value)}
                  placeholder="production"
                  className="text-[13px]"
                />
              </div>
            </div>

            <div>
              <FieldLabel label="Endpoint" />
              <Input
                value={endpoint}
                onChange={(e) => setEndpoint(e.target.value)}
                placeholder="https://<account>.r2.cloudflarestorage.com"
                className="font-mono text-[13px]"
              />
            </div>

            <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
              <div>
                <FieldLabel label="Access key ID" />
                <Input
                  value={accessKeyId}
                  onChange={(e) => setAccessKeyId(e.target.value)}
                  placeholder="access key id"
                  className="font-mono text-[13px]"
                />
              </div>
              <div>
                <FieldLabel label="Secret access key" />
                <Input
                  type="password"
                  value={secret}
                  onChange={(e) => setSecret(e.target.value)}
                  placeholder={
                    secretSet ? '•••••••• (unchanged)' : 'secret key'
                  }
                  className="font-mono text-[13px]"
                />
              </div>
            </div>

            <p className="text-[11px] leading-5 text-text-tertiary">
              slasha puts the database in WAL mode automatically. Backups
              require the web process at a single instance — Litestream must be
              the only writer. For best concurrency, set{' '}
              <span className="font-mono">busy_timeout</span> and{' '}
              <span className="font-mono">synchronous=NORMAL</span> in your app.
            </p>
          </div>
        ) : savedEnabled ? (
          <div className="px-6 py-4 text-[13px] text-text-tertiary">
            Backups will be turned off when you save. The replica in object
            storage is kept.
          </div>
        ) : null}

        {hasBody ? (
          <div className="flex items-center justify-between gap-4 border-t border-border bg-surface/30 px-6 py-4">
            {backup?.enabled && secretSet ? (
              <button
                type="button"
                onClick={() => setShowRestoreConfirm(true)}
                disabled={restoreBackup.isPending}
                className="inline-flex items-center gap-1.5 text-[12px] text-text-tertiary transition-colors hover:text-text disabled:opacity-50"
              >
                <History className="size-3.5" />
                {backup.restore_pending ? 'Restore queued' : 'Restore latest'}
              </button>
            ) : (
              <span />
            )}
            <Button
              label="Save Changes"
              icon={<Save className="size-3.5" />}
              size="sm"
              onClick={handleSave}
              isLoading={saveBackup.isPending}
            />
          </div>
        ) : null}
      </div>

      <ConfirmationDialog
        open={showRestoreConfirm}
        onOpenChange={setShowRestoreConfirm}
        title="Restore database from backup"
        description="On the next deploy, the live database will be discarded and replaced with the latest copy from object storage. This cannot be undone. Continue?"
        confirmLabel="Queue restore"
        onConfirm={handleRestore}
      />
    </VStack>
  );
}
