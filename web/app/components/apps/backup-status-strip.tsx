import { RefreshCw } from 'lucide-react';

import type { BackupStatus, ReplicaHealth } from '~/queries/storage';
import { HStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { formatRelativeTime } from '~/utils/format';

type BackupStatusStripProps = {
  status: BackupStatus;
  health: ReplicaHealth | undefined;
  isChecking: boolean;
  onCheck: () => void;
};

function deriveView(status: BackupStatus, health: ReplicaHealth | undefined) {
  if (status.restore_pending) {
    return { dot: 'bg-amber-500', label: 'Restore queued for next deploy' };
  }
  if (health?.healthy === false) {
    return { dot: 'bg-red-500', label: 'Replication failing' };
  }
  if (status.web_running && health?.healthy) {
    return { dot: 'animate-pulse bg-emerald-500', label: 'Replicating' };
  }
  if (status.web_running) {
    return {
      dot: 'animate-pulse bg-blue-500',
      label: 'Replicating (verifying…)',
    };
  }
  return {
    dot: 'bg-text-tertiary',
    label: 'Idle — deploy to start replicating',
  };
}

export function BackupStatusStrip(props: BackupStatusStripProps) {
  const { status, health, isChecking, onCheck } = props;
  const view = deriveView(status, health);
  const failing = health?.healthy === false;

  return (
    <div className="border-b border-border px-6 py-3">
      <div className="flex items-center justify-between gap-3">
        <HStack space={2}>
          <span className={cn('size-2 rounded-full', view.dot)} />
          <span className="text-[12px] text-text-secondary">{view.label}</span>
          {!failing && health?.last_synced_at ? (
            <span className="text-[12px] text-text-tertiary">
              · last synced {formatRelativeTime(health.last_synced_at)}
            </span>
          ) : null}
        </HStack>
        <button
          type="button"
          onClick={onCheck}
          disabled={isChecking}
          className="inline-flex items-center gap-1 text-[11px] text-text-tertiary transition-colors hover:text-text disabled:opacity-50"
        >
          <RefreshCw className={cn('size-3', isChecking && 'animate-spin')} />
          Check replica
        </button>
      </div>
      {failing && health?.health_error ? (
        <p className="mt-2 break-words text-[11px] leading-5 text-red-400">
          {health.health_error}
        </p>
      ) : null}
    </div>
  );
}
