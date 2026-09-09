import { Suspense, useState } from 'react';
import { Outlet, redirect, useLocation, useParams } from 'react-router';
import { PageSkeleton } from '~/components/global/page-skeleton';
import { CommandMenu } from '~/components/global/command-menu';
import { Sidebar } from '~/components/global/sidebar';
import { TopBar } from '~/components/global/top-bar';
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

// Detail screens (file tree, log stream, deployment list) fill the viewport and
// scroll their own panes, so the shell must not add a scroll container of its own.
function useOwnsScroll() {
  const location = useLocation();
  const params = useParams();

  if (location.pathname.startsWith('/apps/') && params.slug) {
    return true;
  }
  if (location.pathname.startsWith('/nodes/') && params.id) {
    return true;
  }
  return location.pathname.startsWith('/alerts');
}

export default function UserLayout() {
  const ownsScroll = useOwnsScroll();
  const [isCommandMenuOpen, setIsCommandMenuOpen] = useState(false);

  return (
    <div className="flex h-screen bg-bg">
      <Sidebar onSearch={() => setIsCommandMenuOpen(true)} />

      <div className="ml-[240px] flex flex-1 flex-col overflow-hidden">
        <TopBar />
        <main
          className={
            ownsScroll
              ? 'flex min-h-0 flex-1 flex-col overflow-hidden'
              : 'min-h-0 flex-1 overflow-y-auto'
          }
        >
          <Suspense fallback={<PageSkeleton />}>
            <Outlet />
          </Suspense>
        </main>
      </div>

      <CommandMenu
        open={isCommandMenuOpen}
        onOpenChange={setIsCommandMenuOpen}
      />
    </div>
  );
}
