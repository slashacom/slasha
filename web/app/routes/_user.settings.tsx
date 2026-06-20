import { Outlet } from 'react-router';
import { TabNav } from '~/components/interface/tab-nav';

export default function SettingsLayout() {
  const tabs = [
    { label: 'Account', to: '/settings/account' },
    { label: 'SSH Keys', to: '/settings/ssh-keys' },
  ];

  return (
    <div className="flex flex-1 flex-col space-y-8">
      <TabNav items={tabs} />
      <div className="flex-1">
        <Outlet />
      </div>
    </div>
  );
}
