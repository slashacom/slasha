import {
  AlertCircle,
  CheckCircle2,
  CircleDashed,
  Clock,
  XCircle,
  type LucideIcon,
} from 'lucide-react';
import { cn } from '~/utils/classname';

export type StatusKind =
  | 'Pending'
  | 'Building'
  | 'Provisioning'
  | 'Running'
  | 'Failed'
  | 'Stopped';

type StatusConfig = {
  icon: LucideIcon;
  color: string;
  bg: string;
  spin?: boolean;
};

const STATUS_CONFIG: Record<StatusKind, StatusConfig> = {
  Pending: { icon: Clock, color: 'text-text-tertiary', bg: 'bg-white/5' },
  Building: {
    icon: CircleDashed,
    color: 'text-sky-400',
    bg: 'bg-sky-400/10',
    spin: true,
  },
  Provisioning: {
    icon: CircleDashed,
    color: 'text-sky-400',
    bg: 'bg-sky-400/10',
    spin: true,
  },
  Running: {
    icon: CheckCircle2,
    color: 'text-emerald-400',
    bg: 'bg-emerald-400/10',
  },
  Failed: { icon: XCircle, color: 'text-red-400', bg: 'bg-red-400/10' },
  Stopped: { icon: AlertCircle, color: 'text-text-tertiary', bg: 'bg-white/5' },
};

type StatusBadgeProps = {
  status: StatusKind;
};

export function StatusBadge(props: StatusBadgeProps) {
  const { status } = props;
  const config = STATUS_CONFIG[status];
  const Icon = config.icon;

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded px-2 py-0.5 text-[11px] font-medium',
        config.color,
        config.bg
      )}
    >
      <Icon className={cn('size-3', config.spin && 'animate-spin')} />
      {status}
    </span>
  );
}
