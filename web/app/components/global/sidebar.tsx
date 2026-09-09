import { NavLink, useNavigate, useLocation } from 'react-router';

import {
  Bell,
  LayoutGrid,
  Search,
  Server,
  Settings,
  Users,
} from 'lucide-react';

import { useSuspenseQuery } from '@tanstack/react-query';
import { cn } from '~/utils/classname';
import { getAuthMeOptions } from '~/queries/auth';
import { removeAuthToken } from '~/utils/jwt';

type SidebarItemProps = {
  to: string;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
};

function SidebarItem(props: SidebarItemProps) {
  const { to, icon: Icon, label } = props;
  const location = useLocation();
  const isActive = location.pathname.startsWith(to);

  return (
    <NavLink
      to={to}
      className={cn(
        'flex items-center gap-2 py-1.5 text-[14px] transition-colors',
        isActive
          ? 'font-medium text-text'
          : 'text-text-tertiary hover:text-text-secondary'
      )}
    >
      <Icon className="h-4 w-4" />
      {label}
    </NavLink>
  );
}

type SidebarProps = {
  onSearch: () => void;
};

export function Sidebar(props: SidebarProps) {
  const { onSearch } = props;
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getAuthMeOptions());
  const isAdmin = data.user?.role === 'Admin';

  const handleLogout = () => {
    removeAuthToken();
    navigate('/login');
  };

  return (
    <aside className="fixed inset-y-0 left-0 z-50 flex w-[240px] flex-col border-r border-border bg-bg">
      <div className="flex h-12 items-center justify-between border-b border-border px-6">
        <NavLink
          to="/apps"
          className="text-[18px] font-medium tracking-tight !text-text !no-underline"
        >
          slasha
        </NavLink>
        <button
          type="button"
          onClick={onSearch}
          title="Search (⌘K)"
          aria-label="Search"
          aria-keyshortcuts="Meta+K"
          className="flex h-6 cursor-pointer items-center gap-1 rounded border border-border bg-surface px-1.5 text-text-tertiary transition-colors hover:bg-white/[0.06] hover:text-text"
        >
          <Search className="size-3" />
          <kbd className="font-sans text-[11px] tracking-wide">⌘K</kbd>
        </button>
      </div>

      <nav className="flex-1 px-6 pt-5">
        <SidebarItem to="/apps" icon={LayoutGrid} label="Apps" />
        {isAdmin && <SidebarItem to="/nodes" icon={Server} label="Nodes" />}
        {isAdmin && <SidebarItem to="/alerts" icon={Bell} label="Alerts" />}
        {isAdmin && <SidebarItem to="/users" icon={Users} label="Users" />}
        <SidebarItem to="/settings" icon={Settings} label="Settings" />
      </nav>

      <div className="px-6 pb-6">
        <a
          href="https://github.com/slashacom/slasha/issues/new/choose"
          target="_blank"
          rel="noopener noreferrer"
          className="block py-1.5 text-[14px] text-text-tertiary transition-colors hover:text-text-secondary"
        >
          Feedback
        </a>
        <button
          onClick={handleLogout}
          className="block py-1.5 text-[14px] text-text-tertiary transition-colors hover:text-text-secondary"
        >
          Logout
        </button>
      </div>
    </aside>
  );
}
