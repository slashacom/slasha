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
        'flex gap-1.5 disabled:cursor-not-allowed px-2 cursor-pointer items-center justify-center  hover:bg-gray-100 disabled:text-gray-300 disabled:hover:bg-transparent transition-colors',
        isFetching && 'bg-gray-100! hover:bg-gray-100 disabled:bg-gray-100'
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
    <div className="flex justify-star">
      <PaginationButton
        onClick={onPrevPage}
        disabled={disablePrev}
        icon={ChevronLeft}
      />

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button className="flex focus:outline-none focus:ring-0 items-center gap-1.5 px-1.5 py-1.5 text-xs cursor-pointer font-medium text-gray-700 hover:bg-gray-100  transition-colors">
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
