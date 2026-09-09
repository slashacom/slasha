import { useNavigate } from 'react-router';
import { cn } from '~/utils/classname';

export type TableColumn = {
  label: string;
  align?: 'left' | 'right';
  className?: string;
};

type TableProps = {
  columns: Array<string | TableColumn>;
  children: React.ReactNode;
  className?: string;
};

export function Table(props: TableProps) {
  const { columns, children, className } = props;
  return (
    <table
      className={cn(
        'w-full text-left text-sm',
        '[&_td:first-child]:pl-8 [&_th:first-child]:pl-8',
        '[&_td:last-child]:pr-8 [&_th:last-child]:pr-8',
        className
      )}
    >
      <thead>
        <tr className="border-b border-border">
          {columns.map((column, i) => {
            const col: TableColumn =
              typeof column === 'string' ? { label: column } : column;
            return (
              <th
                key={i}
                className={cn(
                  'pb-2 pr-4 text-xs font-medium uppercase tracking-wider text-text-tertiary',
                  col.align === 'right' && 'text-right',
                  col.className
                )}
              >
                {col.label}
              </th>
            );
          })}
        </tr>
      </thead>
      <tbody className="divide-y divide-border">{children}</tbody>
    </table>
  );
}

type TableRowProps = {
  to?: string;
  children: React.ReactNode;
  className?: string;
};

export function TableRow(props: TableRowProps) {
  const { to, children, className } = props;
  const navigate = useNavigate();

  if (!to) {
    return <tr className={className}>{children}</tr>;
  }

  return (
    <tr
      role="link"
      tabIndex={0}
      onClick={() => navigate(to)}
      onKeyDown={(event) => {
        if (event.key !== 'Enter') {
          return;
        }
        navigate(to);
      }}
      className={cn(
        'group cursor-pointer transition-colors hover:bg-white/[0.02] focus-visible:bg-white/[0.02] focus-visible:outline-none',
        className
      )}
    >
      {children}
    </tr>
  );
}
