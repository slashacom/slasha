import {
  AlertCircle,
  CheckCircle2,
  CircleDashed,
  Clock,
  XCircle,
  type LucideIcon,
} from 'lucide-react';
import type { NodeStatus } from '~/models/node';
import type { NodeConnectionStatus } from '~/queries/nodes';
import { cn } from '~/utils/classname';

export type StatusKind =
  | 'Pending'
  | 'Building'
  | 'Provisioning'
  | 'Running'
  | 'Failed'
  | 'Stopped'
  | 'Restoring'
  | 'Backing up'
  | 'Restarting'
  | 'Stopping'
  | 'Deleting'
  | string;

type StatusConfig = {
  icon: LucideIcon;
  color: string;
  bg: string;
  spin?: boolean;
  label: string;
};

const STATUS_CONFIG: Record<string, StatusConfig> = {
  pending: {
    icon: Clock,
    color: 'text-text-tertiary',
    bg: 'bg-white/5',
    label: 'Pending',
  },
  building: {
    icon: CircleDashed,
    color: 'text-sky-400',
    bg: 'bg-sky-400/10',
    spin: true,
    label: 'Building',
  },
  provisioning: {
    icon: CircleDashed,
    color: 'text-sky-400',
    bg: 'bg-sky-400/10',
    spin: true,
    label: 'Provisioning',
  },
  running: {
    icon: CheckCircle2,
    color: 'text-emerald-400',
    bg: 'bg-emerald-400/10',
    label: 'Running',
  },
  failed: {
    icon: XCircle,
    color: 'text-red-400',
    bg: 'bg-red-400/10',
    label: 'Failed',
  },
  stopped: {
    icon: AlertCircle,
    color: 'text-text-tertiary',
    bg: 'bg-white/5',
    label: 'Stopped',
  },
  restoring: {
    icon: CircleDashed,
    color: 'text-amber-400',
    bg: 'bg-amber-400/10',
    spin: true,
    label: 'Restoring',
  },
  'backing up': {
    icon: CircleDashed,
    color: 'text-sky-400',
    bg: 'bg-sky-400/10',
    spin: true,
    label: 'Backing up',
  },
  restarting: {
    icon: CircleDashed,
    color: 'text-sky-400',
    bg: 'bg-sky-400/10',
    spin: true,
    label: 'Restarting',
  },
  stopping: {
    icon: CircleDashed,
    color: 'text-text-tertiary',
    bg: 'bg-white/5',
    spin: true,
    label: 'Stopping',
  },
  deleting: {
    icon: CircleDashed,
    color: 'text-red-400',
    bg: 'bg-red-400/10',
    spin: true,
    label: 'Deleting',
  },
};

type StatusBadgeProps = {
  status: StatusKind;
};

export function StatusBadge(props: StatusBadgeProps) {
  const { status } = props;
  const normalizedKey = status.toLowerCase();
  const config = STATUS_CONFIG[normalizedKey] ?? {
    icon: AlertCircle,
    color: 'text-text-secondary',
    bg: 'bg-white/5',
    label: status,
  };
  const Icon = config.icon;

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 whitespace-nowrap shrink-0 rounded px-2 py-0.5 text-[11px] font-medium capitalize',
        config.color,
        config.bg
      )}
    >
      <Icon className={cn('size-3', config.spin && 'animate-spin')} />
      {config.label}
    </span>
  );
}

type NodeStatusBadgeProps = {
  status: NodeStatus;
  connectionStatus: NodeConnectionStatus;
};

export function NodeStatusBadge(props: NodeStatusBadgeProps) {
  const { status, connectionStatus } = props;

  if (status === 'Ready') {
    if (connectionStatus === 'online') {
      return (
        <span className="inline-flex items-center gap-1.5 whitespace-nowrap shrink-0 rounded border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-400">
          <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500" />
          Online
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1.5 whitespace-nowrap shrink-0 rounded border border-red-500/20 bg-red-500/10 px-2 py-0.5 text-[11px] font-medium text-red-400">
        <span className="h-1.5 w-1.5 rounded-full bg-red-500" />
        Offline
      </span>
    );
  }

  if (status === 'SettingUp') {
    return (
      <span className="inline-flex items-center gap-1.5 whitespace-nowrap shrink-0 rounded border border-amber-500/20 bg-amber-500/10 px-2 py-0.5 text-[11px] font-medium text-amber-400">
        <CircleDashed className="size-3 animate-spin" />
        Setting Up
      </span>
    );
  }

  if (status === 'Deleting') {
    return (
      <span className="inline-flex items-center gap-1.5 whitespace-nowrap shrink-0 rounded border border-red-500/20 bg-red-500/10 px-2 py-0.5 text-[11px] font-medium text-red-400">
        <CircleDashed className="size-3 animate-spin" />
        Deleting
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-1.5 whitespace-nowrap shrink-0 rounded border border-red-500/20 bg-red-500/10 px-2 py-0.5 text-[11px] font-medium text-red-400">
      <AlertCircle className="size-3" />
      Error
    </span>
  );
}
