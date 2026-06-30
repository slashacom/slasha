import { cn } from '~/utils/classname';

type AlertCardProps = {
  children: React.ReactNode;
  className?: string;
};

export function AlertCard(props: AlertCardProps) {
  const { children, className } = props;

  return (
    <div
      className={cn(
        'rounded-lg border border-border bg-surface p-6',
        className
      )}
    >
      {children}
    </div>
  );
}
