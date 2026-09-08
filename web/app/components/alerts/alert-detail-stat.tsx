import { cn } from '~/utils/classname';

type AlertDetailStatProps = {
  label: string;
  value: React.ReactNode;
  mono?: boolean;
  valueClassName?: string;
};

export function AlertDetailStat(props: AlertDetailStatProps) {
  const { label, value, mono, valueClassName } = props;

  return (
    <div className="min-w-0 rounded-md border border-border bg-bg/40 p-3">
      <p className="text-xs font-medium text-text-tertiary">{label}</p>
      <div
        className={cn(
          'mt-1',
          mono
            ? 'break-all font-mono text-xs font-medium tracking-normal text-text-secondary'
            : 'break-words text-pretty text-sm font-semibold tracking-tight text-text',
          valueClassName
        )}
      >
        {value}
      </div>
    </div>
  );
}
