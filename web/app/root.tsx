import { Links, Meta, Outlet, Scripts, ScrollRestoration } from 'react-router';
import type { Route } from './+types/root';

import { Toaster } from 'sonner';
import { QueryClientProvider } from '@tanstack/react-query';

import { queryClient } from '~/utils/query-client';

import { ErrorView } from '~/components/global/error-view';
import { FullPageSpinner } from '~/components/icons/spinner';
import { NavigationProgress } from '~/components/interface/navigation-progress';

import './styles/global.css';

export const links: Route.LinksFunction = () => [
  { rel: 'preconnect', href: 'https://fonts.googleapis.com' },
  {
    rel: 'preconnect',
    href: 'https://fonts.gstatic.com',
    crossOrigin: 'anonymous',
  },
  {
    rel: 'stylesheet',
    href: 'https://fonts.googleapis.com/css2?family=Geist:wght@100..900&family=Geist+Mono:wght@100..900&display=swap',
  },
];

export function Layout({ children }: { children: React.ReactNode }) {
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
  return <FullPageSpinner />;
}
