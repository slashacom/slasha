import { cn } from '~/utils/classname';

type AlertStatusBadgeProps = {
  children: React.ReactNode;
  state: 'ok' | 'warn' | 'muted';
};

const stateClasses = {
  ok: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300',
  warn: 'border-red-500/30 bg-red-500/10 text-red-300',
  muted: 'border-border bg-surface text-text-tertiary',
} as const;

export function AlertStatusBadge(props: AlertStatusBadgeProps) {
  const { children, state } = props;

  return (
    <span
      className={cn(
        'inline-flex rounded-full border px-2 py-0.5 text-xs',
        stateClasses[state]
      )}
    >
      {children}
    </span>
  );
}
