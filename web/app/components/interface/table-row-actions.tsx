import { Link } from 'react-router';
import { cn } from '~/utils/classname';

export type TableRowAction = {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  to?: string;
  onClick?: () => void;
  isDestructive?: boolean;
  isDisabled?: boolean;
};

type TableRowActionsProps = {
  actions: TableRowAction[];
};

export function TableRowActions(props: TableRowActionsProps) {
  const { actions } = props;

  return (
    <div className="flex items-center justify-end gap-3">
      {actions.map((action) => {
        const {
          label,
          icon: Icon,
          to,
          onClick,
          isDestructive,
          isDisabled,
        } = action;
        const className = cn(
          'transition-colors',
          isDestructive
            ? 'text-red-400/80 hover:text-red-400'
            : 'text-text-secondary hover:text-text',
          isDisabled && 'pointer-events-none opacity-50'
        );

        if (to) {
          return (
            <Link
              key={label}
              to={to}
              title={label}
              aria-label={label}
              className={cn(className, '!no-underline')}
            >
              <Icon className="size-4" />
            </Link>
          );
        }

        return (
          <button
            key={label}
            type="button"
            title={label}
            aria-label={label}
            onClick={onClick}
            disabled={isDisabled}
            className={className}
          >
            <Icon className="size-4" />
          </button>
        );
      })}
    </div>
  );
}
