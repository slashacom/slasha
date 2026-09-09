import { Outlet } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { TabNav } from '~/components/interface/tab-nav';
import { TabActionsProvider } from '~/components/interface/tab-actions';
import { getAuthMeOptions } from '~/queries/auth';

export default function SettingsLayout() {
  useSuspenseQuery(getAuthMeOptions());

  const tabs = [
    { label: 'Account', to: '/settings/account' },
    { label: 'Connections', to: '/settings/connections' },
    { label: 'SSH Keys', to: '/settings/ssh-keys' },
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TabActionsProvider>
        {(slot) => (
          <>
            <TabNav
              className="shrink-0 bg-surface/30 px-8"
              actions={slot}
              items={tabs}
            />

            <div className="min-h-0 flex-1">
              <Outlet />
            </div>
          </>
        )}
      </TabActionsProvider>
    </div>
  );
}
