import {
  CheckCircle2,
  CircleDashed,
  Loader2,
  XCircle,
  Database,
  Activity,
  RefreshCw,
  Trash2,
  type LucideIcon,
} from 'lucide-react';
import type { AppRuntimeStatus } from '~/queries/apps';
import { cn } from '~/utils/classname';

type AppRuntimeBadgeProps = {
  status: AppRuntimeStatus;
};

const STATUS_STYLES: Record<
  AppRuntimeStatus,
  { label: string; icon: LucideIcon; className: string; spin?: boolean }
> = {
  running: {
    label: 'Live',
    icon: CheckCircle2,
    className: 'border-emerald-500/20 bg-emerald-500/10 text-emerald-400',
  },
  deploying: {
    label: 'Deploying',
    icon: Loader2,
    className: 'border-sky-500/20 bg-sky-500/10 text-sky-400',
    spin: true,
  },
  failed: {
    label: 'Failed',
    icon: XCircle,
    className: 'border-red-500/20 bg-red-500/10 text-red-400',
  },
  idle: {
    label: 'Idle',
    icon: CircleDashed,
    className: 'border-border bg-white/5 text-text-tertiary',
  },
  migrating: {
    label: 'Migrating',
    icon: Database,
    className: 'border-purple-500/20 bg-purple-500/10 text-purple-400',
    spin: false,
  },
  scaling: {
    label: 'Scaling',
    icon: Activity,
    className: 'border-indigo-500/20 bg-indigo-500/10 text-indigo-400',
    spin: false,
  },
  syncing: {
    label: 'Syncing',
    icon: RefreshCw,
    className: 'border-amber-500/20 bg-amber-500/10 text-amber-400',
    spin: true,
  },
  purging: {
    label: 'Purging',
    icon: Trash2,
    className: 'border-rose-500/20 bg-rose-500/10 text-rose-400',
    spin: false,
  },
};

export function AppRuntimeBadge(props: AppRuntimeBadgeProps) {
  const { status } = props;
  const style = STATUS_STYLES[status] || STATUS_STYLES.idle;
  const Icon = style.icon;

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium',
        style.className
      )}
    >
      <Icon
        className={cn(
          'size-3',
          style.spin && 'animate-spin',
          !style.spin &&
            ['deploying', 'migrating', 'scaling', 'purging'].includes(status) &&
            'animate-pulse'
        )}
      />
      {style.label}
    </span>
  );
}
