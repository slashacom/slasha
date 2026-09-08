import { cn } from '~/utils/classname';

type AlertCardProps = {
  title?: string;
  description?: string;
  children: React.ReactNode;
  className?: string;
};

export function AlertCard(props: AlertCardProps) {
  const { title, description, children, className } = props;

  return (
    <div
      className={cn(
        'min-w-0 rounded-lg border border-border bg-surface p-6',
        className
      )}
    >
      {title ? (
        <div className="mb-4">
          <h3 className="text-xs font-medium text-text-tertiary">{title}</h3>
          {description ? (
            <p className="mt-1 max-w-prose text-pretty text-[11px] text-text-tertiary">
              {description}
            </p>
          ) : null}
        </div>
      ) : null}
      {children}
    </div>
  );
}
