import {
  CheckCircle2,
  CircleDashed,
  Loader2,
  XCircle,
  type LucideIcon,
} from 'lucide-react';
import type { AppStatusTone, AppStatusView } from '~/utils/app-status';
import { cn } from '~/utils/classname';

type AppRuntimeBadgeProps = {
  status: AppStatusView;
};

const TONE_STYLES: Record<
  AppStatusTone,
  { icon: LucideIcon; className: string; spin?: boolean }
> = {
  live: {
    icon: CheckCircle2,
    className: 'border-emerald-500/20 bg-emerald-500/10 text-emerald-400',
  },
  deploying: {
    icon: Loader2,
    className: 'border-sky-500/20 bg-sky-500/10 text-sky-400',
    spin: true,
  },
  failed: {
    icon: XCircle,
    className: 'border-red-500/20 bg-red-500/10 text-red-400',
  },
  idle: {
    icon: CircleDashed,
    className: 'border-border bg-white/5 text-text-tertiary',
  },
  migrating: {
    icon: Loader2,
    className: 'border-amber-500/20 bg-amber-500/10 text-amber-400',
    spin: true,
  },
};

export function AppRuntimeBadge(props: AppRuntimeBadgeProps) {
  const { status } = props;
  const tone = TONE_STYLES[status.tone];
  const Icon = tone.icon;

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium',
        tone.className
      )}
    >
      <Icon className={cn('size-3', tone.spin && 'animate-spin')} />
      {status.label}
    </span>
  );
}
