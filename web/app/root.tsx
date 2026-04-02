import { Links, Meta, Outlet, Scripts, ScrollRestoration } from 'react-router';
import type { Route } from './+types/root';
import { ErrorView } from '~/components/global/error-view';
import { Loader } from '~/components/icons/loader';

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
    href: 'https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&display=swap',
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
        {children}
        <ScrollRestoration />
        <Scripts />
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
