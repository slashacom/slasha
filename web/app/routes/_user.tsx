import { Suspense } from 'react';
import { Outlet, redirect, useLocation, useParams } from 'react-router';
import { Sidebar } from '~/components/global/sidebar';
import { getAuthMeOptions } from '~/queries/auth';
import { queryClient } from '~/utils/query-client';
import { isLoggedIn } from '~/utils/jwt';

// Ensure the logged-in user's data is in the React Query cache before the
// layout (sidebar, user menu) renders. Without this, useQuery initially
// returns undefined data and the user menu flickers in once the request
// resolves.
export async function clientLoader() {
  if (!isLoggedIn()) {
    throw redirect('/login');
  }

  await queryClient.ensureQueryData(getAuthMeOptions());
  return null;
}

function usePageTitle() {
  const location = useLocation();
  const params = useParams();
  const path = location.pathname;

  if (path === '/apps' || path === '/apps/') {
    return 'Apps';
  }
  if (path.startsWith('/apps/') && params.slug) {
    return 'Apps';
  }
  if (path.startsWith('/monitoring')) {
    return 'Monitoring';
  }
  if (path === '/alerts' || path.startsWith('/alerts/')) {
    return 'Alerts';
  }
  if (path === '/users' || path === '/users/') {
    return 'Users';
  }
  if (path === '/users/new') {
    return 'Users';
  }
  if (path.startsWith('/users/')) {
    return 'Users';
  }
  if (path.startsWith('/settings/')) {
    return 'Settings';
  }
  if (path === '/nodes' || path === '/nodes/' || path.startsWith('/nodes/')) {
    return 'Nodes';
  }
  return '';
}

export default function UserLayout() {
  const title = usePageTitle();
  const location = useLocation();
  const params = useParams();
  const isFullWidth =
    (!!params.slug && location.pathname.startsWith('/apps/')) ||
    location.pathname.startsWith('/monitoring') ||
    location.pathname.startsWith('/alerts') ||
    (location.pathname.startsWith('/nodes/') && !!params.id);

  return (
    <div className="flex h-screen bg-bg">
      <Sidebar />

      <div className="ml-[240px] flex flex-1 flex-col overflow-hidden">
        <header className="flex h-12 shrink-0 items-center border-b border-border px-8">
          <span className="text-[13px] text-text-tertiary">{title}</span>
        </header>
        {isFullWidth ? (
          <main className="flex flex-1 flex-col overflow-hidden">
            <Suspense fallback={null}>
              <Outlet />
            </Suspense>
          </main>
        ) : (
          <main className="flex-1 overflow-y-auto p-6">
            <div className="max-w-4xl">
              <Suspense fallback={null}>
                <Outlet />
              </Suspense>
            </div>
          </main>
        )}
      </div>
    </div>
  );
}
