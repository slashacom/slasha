import { HStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type SettingsCardProps = {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description?: React.ReactNode;
  actions?: React.ReactNode;
  children?: React.ReactNode;
  body?: React.ReactNode;
  className?: string;
};

export function SettingsCard(props: SettingsCardProps) {
  const {
    icon: Icon,
    title,
    description,
    actions,
    children,
    body,
    className,
  } = props;

  return (
    <div
      className={cn(
        'min-w-0 overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm',
        className
      )}
    >
      <HStack
        justifyContent="between"
        alignItems="start"
        className={cn(
          'gap-4 px-6 py-5',
          body != null && 'border-b border-border bg-surface/50'
        )}
      >
        <HStack space={3} alignItems="start" className="min-w-0">
          <div className="shrink-0 rounded-lg bg-white/5 p-2 text-text-secondary">
            <Icon className="size-5" />
          </div>
          <div className="min-w-0">
            <h3 className="text-balance text-[15px] font-semibold text-text">
              {title}
            </h3>
            {description ? (
              <p className="mt-0.5 max-w-prose text-pretty text-[13px] text-text-tertiary">
                {description}
              </p>
            ) : null}
            {children ? <div className="mt-4">{children}</div> : null}
          </div>
        </HStack>
        {actions ? <div className="shrink-0">{actions}</div> : null}
      </HStack>

      {body != null ? <div className="min-w-0">{body}</div> : null}
    </div>
  );
}
