import { NavLink, useNavigate } from 'react-router';
import { LayoutGrid, Users, LogOut } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { cn } from '~/utils/classname';
import { VStack } from '~/components/interface/stacks';
import { getAuthMeOptions } from '~/queries/auth';
import { removeAuthToken } from '~/utils/jwt';

interface SidebarItemProps {
  to: string;
  icon: React.ReactNode;
  label: string;
}

function SidebarItem({ to, icon, label }: SidebarItemProps) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(
          'flex items-center gap-3 px-3 py-2 text-sm font-medium transition-all hover:bg-black/5 rounded-lg',
          isActive
            ? 'text-black font-semibold bg-black/5'
            : 'text-neutral-500 hover:text-black'
        )
      }
    >
      {icon}
      {label}
    </NavLink>
  );
}

export function Sidebar() {
  const navigate = useNavigate();
  const { data } = useQuery(getAuthMeOptions());
  const isAdmin = data?.user?.role === 'admin';

  const handleLogout = () => {
    removeAuthToken();
    navigate('/login');
  };

  return (
    <div className="flex h-screen w-64 flex-col border-r border-neutral-100 bg-neutral-50 p-4">
      <VStack space={6} className="h-full">
        <div className="px-2 py-4">
          <h1 className="text-xl font-bold tracking-tight text-black">
            slasha
          </h1>
        </div>

        <nav className="flex-grow">
          <VStack space={1}>
            <SidebarItem
              to="/apps"
              icon={<LayoutGrid className="size-4" />}
              label="Apps"
            />
            {isAdmin && (
              <SidebarItem
                to="/users"
                icon={<Users className="size-4" />}
                label="Users"
              />
            )}
          </VStack>
        </nav>

        <button
          onClick={handleLogout}
          className="flex items-center gap-3 px-3 py-2 text-sm font-medium text-neutral-500 transition-all hover:bg-red-50 hover:text-red-600 rounded-lg w-full text-left"
        >
          <LogOut className="size-4" />
          Logout
        </button>
      </VStack>
    </div>
  );
}
