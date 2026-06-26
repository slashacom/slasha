import {
  CheckCircle2,
  CircleDashed,
  Loader2,
  XCircle,
  type LucideIcon,
} from 'lucide-react';
import type { HealthStatus } from '~/models/domain-health';
import { cn } from '~/utils/classname';

type DomainHealthBadgeProps = {
  status: HealthStatus | 'checking';
};

const STATUS_STYLES: Record<
  HealthStatus | 'checking',
  { label: string; icon: LucideIcon; className: string; spin?: boolean }
> = {
  healthy: {
    label: 'Healthy',
    icon: CheckCircle2,
    className: 'border-emerald-500/20 bg-emerald-500/10 text-emerald-400',
  },
  pending: {
    label: 'Provisioning',
    icon: Loader2,
    className: 'border-amber-500/20 bg-amber-500/10 text-amber-400',
    spin: true,
  },
  error: {
    label: 'Needs attention',
    icon: XCircle,
    className: 'border-red-500/20 bg-red-500/10 text-red-400',
  },
  unknown: {
    label: 'Unknown',
    icon: CircleDashed,
    className: 'border-border bg-white/5 text-text-tertiary',
  },
  checking: {
    label: 'Checking',
    icon: Loader2,
    className: 'border-border bg-white/5 text-text-tertiary',
    spin: true,
  },
};

export function DomainHealthBadge(props: DomainHealthBadgeProps) {
  const { status } = props;
  const style = STATUS_STYLES[status];
  const Icon = style.icon;

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium',
        style.className
      )}
    >
      <Icon className={cn('size-3', style.spin && 'animate-spin')} />
      {style.label}
    </span>
  );
}
