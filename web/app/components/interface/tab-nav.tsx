import { NavLink } from 'react-router';
import { cn } from '~/utils/classname';

export type TabNavItem = {
  label: string;
  to: string;
  end?: boolean;
};

type TabNavProps = {
  items: TabNavItem[];
  className?: string;
};

export function TabNav(props: TabNavProps) {
  const { items, className } = props;
  return (
    <div className={cn('border-b border-border', className)}>
      <nav className="-mb-px flex gap-6" aria-label="Tabs">
        {items.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.end}
            className={({ isActive }) =>
              cn(
                'flex h-10 items-center whitespace-nowrap border-b-2 text-[13px] font-medium transition-colors',
                isActive
                  ? 'border-white text-text'
                  : 'border-transparent text-text-tertiary hover:text-text-secondary'
              )
            }
          >
            {item.label}
          </NavLink>
        ))}
      </nav>
    </div>
  );
}
