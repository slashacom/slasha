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
    <table className={cn('w-full text-left text-sm', className)}>
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
