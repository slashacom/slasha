import { NavLink, useNavigate, useLocation } from 'react-router';

import { Users, Key, Settings, Activity } from 'lucide-react';

import { LayoutGrid } from '../icons/layout';

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

export function Sidebar() {
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getAuthMeOptions());
  const isAdmin = data.user?.role === 'Admin';

  const handleLogout = () => {
    removeAuthToken();
    navigate('/login');
  };

  return (
    <aside className="fixed inset-y-0 left-0 z-50 flex w-[240px] flex-col border-r border-border bg-bg">
      <div className="flex h-12 items-center border-b border-border px-6">
        <NavLink
          to="/apps"
          className="text-[18px] font-medium tracking-tight !text-text !no-underline"
        >
          slasha
        </NavLink>
      </div>

      <nav className="flex-1 px-6 pt-5">
        <SidebarItem to="/apps" icon={LayoutGrid} label="Apps" />
        <SidebarItem to="/monitoring" icon={Activity} label="Monitoring" />
        {isAdmin && <SidebarItem to="/users" icon={Users} label="Users" />}
        <SidebarItem to="/settings" icon={Settings} label="Settings" />
      </nav>

      <div className="px-6 pb-6">
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
