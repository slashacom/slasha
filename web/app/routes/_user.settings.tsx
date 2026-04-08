import { NavLink, Outlet } from 'react-router';
import { cn } from '~/utils/classname';

export default function SettingsLayout() {
  const tabs = [
    { name: 'Account', to: '/settings/account' },
    { name: 'SSH Keys', to: '/settings/ssh-keys' },
  ];

  return (
    <div className="flex flex-1 flex-col space-y-8">
      <div className="border-b border-border">
        <nav className="-mb-px flex space-x-8" aria-label="Tabs">
          {tabs.map((tab) => (
            <NavLink
              key={tab.name}
              to={tab.to}
              className={({ isActive }) =>
                cn(
                  'whitespace-nowrap border-b-2 py-4 px-1 text-sm font-medium transition-colors',
                  isActive
                    ? 'border-white text-text'
                    : 'border-transparent text-text-tertiary hover:border-text-tertiary hover:text-text-secondary'
                )
              }
            >
              {tab.name}
            </NavLink>
          ))}
        </nav>
      </div>
      <div className="flex-1">
        <Outlet />
      </div>
    </div>
  );
}
