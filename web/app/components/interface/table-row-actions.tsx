import { MoreHorizontal } from 'lucide-react';
import { useNavigate } from 'react-router';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '~/components/interface/dropdown-menu';

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
  const navigate = useNavigate();

  return (
    <div
      className="flex justify-end"
      onClick={(event) => event.stopPropagation()}
    >
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            aria-label="Row actions"
            className="flex size-7 items-center justify-center rounded-md text-text-tertiary opacity-60 transition-all hover:bg-white/5 hover:text-text group-hover:opacity-100 data-[state=open]:bg-white/5 data-[state=open]:text-text data-[state=open]:opacity-100"
          >
            <MoreHorizontal className="size-4" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {actions.map((action) => {
            const {
              label,
              icon: Icon,
              to,
              onClick,
              isDestructive,
              isDisabled,
            } = action;

            return (
              <DropdownMenuItem
                key={label}
                disabled={isDisabled}
                variant={isDestructive ? 'destructive' : 'default'}
                onClick={() => {
                  if (to) {
                    navigate(to);
                    return;
                  }
                  onClick?.();
                }}
              >
                <Icon className="size-3.5" />
                {label}
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
