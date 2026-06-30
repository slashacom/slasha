import { cn } from '~/utils/classname';

type AlertDetailFieldProps = {
  label: string;
  value: React.ReactNode;
  valueClassName?: string;
};

export function AlertDetailField(props: AlertDetailFieldProps) {
  const { label, value, valueClassName } = props;

  return (
    <div className="space-y-1 rounded-md border border-border bg-bg/40 p-3">
      <p className="text-xs font-medium text-text-tertiary">{label}</p>
      <div
        className={cn(
          'text-base font-semibold tracking-tight text-text',
          valueClassName
        )}
      >
        {value}
      </div>
    </div>
  );
}
