import { Outlet, redirect } from 'react-router';
import { TabNav } from '~/components/interface/tab-nav';
import { TabActionsProvider } from '~/components/interface/tab-actions';
import { getAuthMeOptions } from '~/queries/auth';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    throw redirect('/apps');
  }

  return null;
}

export function meta() {
  return [{ title: 'Alerts · slasha' }];
}

export default function AlertsLayout() {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TabActionsProvider>
        {(slot) => (
          <>
            <TabNav
              className="shrink-0 bg-surface/30 px-8"
              actions={slot}
              items={[
                { label: 'Incidents', to: '/alerts', end: true },
                { label: 'Channels', to: '/alerts/channels' },
                { label: 'Rules', to: '/alerts/rules' },
              ]}
            />

            <div className="min-h-0 flex-1 overflow-y-auto">
              <Outlet />
            </div>
          </>
        )}
      </TabActionsProvider>
    </div>
  );
}
