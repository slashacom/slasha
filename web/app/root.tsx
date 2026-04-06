import {
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
  useLocation,
  useNavigate,
} from 'react-router';
import type { Route } from './+types/root';

import { Toaster } from 'sonner';
import { QueryClientProvider } from '@tanstack/react-query';

import { queryClient } from '~/utils/query-client';

import { ErrorView } from '~/components/global/error-view';
import { Loader } from '~/components/icons/loader';
import { NavigationProgress } from '~/components/interface/navigation-progress';

import './styles/global.css';
import { useEffect } from 'react';
import { isLoggedIn } from './utils/jwt';

export const links: Route.LinksFunction = () => [];

const guestRoutes = ['/', '/login', '/register'];

export function Layout({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const navigate = useNavigate();

  useEffect(() => {
    const isGuestRoute = guestRoutes.includes(location.pathname);
    const isUser = isLoggedIn();

    if (isUser && isGuestRoute) {
      navigate('/apps');
      return;
    }

    if (!isUser && !isGuestRoute) {
      navigate('/login');
      return;
    }
  }, [location.pathname]);

  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
      </head>
      <body>
        <QueryClientProvider client={queryClient}>
          {children}
          <ScrollRestoration />
          <Scripts />
          <Toaster
            position="bottom-center"
            richColors
            className="flex w-full items-center justify-center rounded-lg bg-zinc-800 px-5 py-2 text-zinc-200"
            offset={{
              top: 15,
            }}
            visibleToasts={1}
            toastOptions={{
              className: '!w-fit !bg-zinc-800 !text-zinc-300 !border-zinc-900',
              style: {
                width: 'fit-content',
                maxWidth: 'fit-content',
                padding: '8px 15px',
              },
            }}
          />
          <NavigationProgress />
        </QueryClientProvider>
      </body>
    </html>
  );
}

export default function App() {
  return <Outlet />;
}

export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  return (
    <div className="flex items-center justify-center flex-grow h-screen">
      <ErrorView error={error} />
    </div>
  );
}

export function HydrateFallback() {
  return (
    <div className="bg-opacity-75 fixed top-0 left-0 z-[100] flex h-full w-full items-center justify-center bg-white">
      <div className="flex items-center justify-center rounded-lg border border-gray-200 bg-white px-4 py-2">
        <Loader className="size-4 text-gray-500" />
        <span className="ml-2 text-sm text-gray-500">
          please wait&nbsp;
          <span className="animate-pulse">...</span>
        </span>
      </div>
    </div>
  );
}
