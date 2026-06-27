import { Outlet } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { TabNav } from '~/components/interface/tab-nav';
import { getAuthMeOptions } from '~/queries/auth';

export default function SettingsLayout() {
  const { data } = useSuspenseQuery(getAuthMeOptions());

  const tabs = [
    { label: 'Account', to: '/settings/account' },
    { label: 'SSH Keys', to: '/settings/ssh-keys' },
  ];

  if (data.user.role === 'Admin') {
    tabs.push({ label: 'Alerts', to: '/settings/alerts' });
  }

  return (
    <div className="flex flex-1 flex-col space-y-8">
      <TabNav items={tabs} />
      <div className="flex-1">
        <Outlet />
      </div>
    </div>
  );
}
