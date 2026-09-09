import { useQuery } from '@tanstack/react-query';
import { HardDrive, Loader2 } from 'lucide-react';

import { getVolumesOptions } from '~/queries/storage';
import { EmptyPage } from '~/components/global/empty-page';
import { HStack } from '~/components/interface/stacks';
import { formatFileSize } from '~/utils/format';
import { SettingsCard } from '~/components/interface/settings-card';

type StorageManagerProps = {
  appSlug: string;
};

export function StorageManager(props: StorageManagerProps) {
  const { appSlug } = props;
  const { data, isLoading } = useQuery(getVolumesOptions(appSlug));

  const volumes = data?.volumes ?? [];

  const body = (
    <div className="p-6">
      {isLoading ? (
        <div className="flex h-24 items-center justify-center text-text-tertiary">
          <Loader2 className="size-5 animate-spin" />
        </div>
      ) : volumes.length === 0 ? (
        <EmptyPage
          icon={HardDrive}
          size="sm"
          title="No persistent storage."
          subtitle="Mount a volume to keep files on disk across deployments and restarts."
        />
      ) : (
        <div className="divide-y divide-border rounded-lg border border-border bg-surface/20">
          {volumes.map((volume) => (
            <div key={volume.path} className="px-4 py-3">
              <HStack justifyContent="between">
                <HStack space={3}>
                  <HardDrive className="size-3.5 text-text-tertiary" />
                  <span className="font-mono text-[13px] text-text">
                    {volume.path}
                  </span>
                  {volume.is_managed ? (
                    <span className="rounded border border-border bg-white/5 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-text-tertiary">
                      managed
                    </span>
                  ) : null}
                </HStack>
                <span className="text-[12px] text-text-tertiary">
                  {volume.size_bytes == null
                    ? 'persisted'
                    : formatFileSize(volume.size_bytes)}
                </span>
              </HStack>
            </div>
          ))}
        </div>
      )}

      <p className="mt-4 text-[11px] leading-5 text-text-tertiary">
        The managed <span className="font-mono text-text-secondary">/data</span>{' '}
        volume is mounted into every process. Reference it in your env config
        with{' '}
        <span className="font-mono text-text-secondary">
          {'${{ SLASHA.data_dir }}'}
        </span>
        . Write databases and uploads there to keep them across deploys.
      </p>
    </div>
  );

  return (
    <SettingsCard
      icon={HardDrive}
      title="Storage"
      description="Persistent volumes that survive redeploys. Everything else on the container filesystem is reset on each deploy."
      body={body}
    />
  );
}
