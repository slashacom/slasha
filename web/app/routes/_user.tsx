import { Outlet, useLocation, useParams } from 'react-router';
import { Sidebar } from '~/components/global/sidebar';

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
  return '';
}

export default function UserLayout() {
  const title = usePageTitle();
  const location = useLocation();
  const params = useParams();
  const isFullWidth = !!params.slug && location.pathname.startsWith('/apps/');

  return (
    <div className="flex h-screen bg-bg">
      <Sidebar />

      <div className="ml-[240px] flex flex-1 flex-col overflow-hidden">
        <header className="flex h-12 shrink-0 items-center border-b border-border px-8">
          <span className="text-[13px] text-text-tertiary">{title}</span>
        </header>
        {isFullWidth ? (
          <main className="flex flex-1 flex-col overflow-hidden">
            <Outlet />
          </main>
        ) : (
          <main className="flex-1 overflow-y-auto p-6">
            <div className="max-w-4xl">
              <Outlet />
            </div>
          </main>
        )}
      </div>
    </div>
  );
}
