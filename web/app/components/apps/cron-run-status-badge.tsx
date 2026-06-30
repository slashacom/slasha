import type { CronRunStatus } from '~/models/cron';
import { cn } from '~/utils/classname';

type CronRunStatusBadgeProps = {
  status: CronRunStatus;
};

const STATUS: Record<CronRunStatus, { label: string; className: string }> = {
  pending: {
    label: 'Pending',
    className: 'border-border bg-surface text-text-tertiary',
  },
  running: {
    label: 'Running',
    className: 'border-sky-500/30 bg-sky-500/10 text-sky-300',
  },
  succeeded: {
    label: 'Succeeded',
    className: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300',
  },
  failed: {
    label: 'Failed',
    className: 'border-red-500/30 bg-red-500/10 text-red-300',
  },
  timed_out: {
    label: 'Timed out',
    className: 'border-amber-500/30 bg-amber-500/10 text-amber-300',
  },
  skipped: {
    label: 'Skipped',
    className: 'border-border bg-surface text-text-tertiary',
  },
};

export function CronRunStatusBadge(props: CronRunStatusBadgeProps) {
  const { status } = props;
  const { label, className } = STATUS[status];

  return (
    <span
      className={cn(
        'inline-flex rounded-full border px-2 py-0.5 text-xs',
        className
      )}
    >
      {label}
    </span>
  );
}
