import { useState } from 'react';
import { useNavigate } from 'react-router';
import {
  AlertTriangle,
  ArrowLeftIcon,
  ChevronDownIcon,
  ChevronUpIcon,
} from 'lucide-react';
import { isRouteErrorResponse } from 'react-router';

interface ErrorViewProps {
  error: unknown;
}

export function ErrorView({ error }: ErrorViewProps) {
  let title = 'Something went wrong';
  let message = 'An unexpected error occurred. Please try again.';
  let stack: string | undefined;

  if (isRouteErrorResponse(error)) {
    if (error.status === 404) {
      title = 'Page not found';
      message = 'The page you are looking for does not exist.';
    } else if (error.status === 401) {
      title = 'Unauthorized';
      message = 'You are not authorized to access this page.';
    } else if (error.status === 403) {
      title = 'Forbidden';
      message = 'You are not allowed to access this page.';
    } else if (error.status === 400) {
      title = 'Bad request';
      message = error.data.message;
    }
  } else if (error && error instanceof Error) {
    message = error.message;
    if (import.meta.env.DEV) {
      stack = error.stack;
    }
  }

  const [showStack, setShowStack] = useState(false);
  const navigate = useNavigate();

  return (
    <div className="flex h-full w-full flex-col items-center justify-center p-8">
      <div className="flex max-w-md flex-col items-center text-center">
        <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-red-50">
          <AlertTriangle className="size-6 text-red-500" />
        </div>

        <h1 className="mb-1 text-base font-semibold text-neutral-900">
          {title}
        </h1>
        <p className="text-sm text-neutral-500 leading-relaxed">{message}</p>

        {stack && (
          <div className="mt-3 w-full">
            <button
              onClick={() => setShowStack(!showStack)}
              className="flex cursor-pointer items-center gap-1 text-xs text-neutral-400 hover:text-neutral-600 mx-auto"
            >
              {showStack ? (
                <ChevronUpIcon className="size-3" />
              ) : (
                <ChevronDownIcon className="size-3" />
              )}
              {showStack ? 'Hide details' : 'Show details'}
            </button>
            {showStack && (
              <pre className="mt-2 max-h-40 overflow-auto rounded-lg bg-neutral-50 p-3 text-left text-[11px] leading-relaxed text-neutral-600 border border-neutral-200">
                {stack}
              </pre>
            )}
          </div>
        )}

        <button
          className="mt-5 flex cursor-pointer items-center gap-1.5 rounded-lg border border-neutral-200 px-3 py-1.5 text-sm font-medium text-neutral-700 transition-colors hover:bg-neutral-50"
          onClick={() => navigate(-1)}
        >
          <ArrowLeftIcon className="size-3.5" />
          Go back
        </button>
      </div>
    </div>
  );
}
