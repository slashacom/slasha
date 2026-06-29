import {
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  RefreshCw,
} from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '~/components/interface/dropdown-menu';
import { Loader } from '~/components/icons/loader';
import { cn } from '~/utils/classname';

type PaginationButtonProps = {
  onClick?: () => void;
  icon: React.ElementType;
  disabled?: boolean;
  isFetching?: boolean;
  label?: string;
};

function PaginationButton(props: PaginationButtonProps) {
  const { onClick, icon: Icon, disabled, isFetching, label } = props;

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={cn(
        'flex cursor-pointer items-center justify-center gap-1.5 rounded-md px-2 py-1.5 text-text-secondary transition-colors hover:bg-surface hover:text-text disabled:cursor-not-allowed disabled:text-text-tertiary disabled:hover:bg-transparent',
        isFetching && 'bg-surface hover:bg-surface disabled:bg-surface'
      )}
    >
      {isFetching ? (
        <Loader className="size-3.5" />
      ) : (
        <Icon className="size-3.5" />
      )}

      {label && <span className="text-xs">{label}</span>}
    </button>
  );
}

type TablePaginationProps = {
  onPrevPage?: () => void;
  onNextPage?: () => void;
  disablePrev?: boolean;
  disableNext?: boolean;
  limit: number;
  onLimitChange: (size: number) => void;
  isFetchingNextPage?: boolean;
  onRefresh?: () => void;
};

export function TablePagination(props: TablePaginationProps) {
  const {
    onPrevPage,
    onNextPage,
    disablePrev,
    disableNext,
    limit,
    onLimitChange,
    isFetchingNextPage,
    onRefresh,
  } = props;

  return (
    <div className="flex items-center gap-1">
      <PaginationButton
        onClick={onPrevPage}
        disabled={disablePrev}
        icon={ChevronLeft}
      />

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button className="flex cursor-pointer items-center gap-1.5 rounded-md px-2 py-1.5 text-xs font-medium text-text-secondary transition-colors hover:bg-surface hover:text-text focus:outline-none focus:ring-0">
            {limit} rows
            <ChevronDown className="size-3.5" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          {[10, 25, 50, 100].map((size) => (
            <DropdownMenuItem
              className="text-xs"
              key={size}
              onSelect={() => onLimitChange(size)}
            >
              {size}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>

      <PaginationButton
        onClick={onNextPage}
        disabled={disableNext || isFetchingNextPage}
        icon={ChevronRight}
        isFetching={isFetchingNextPage}
      />

      {onRefresh && (
        <PaginationButton
          onClick={onRefresh}
          icon={RefreshCw}
          label="Refresh"
          disabled={isFetchingNextPage}
        />
      )}
    </div>
  );
}
