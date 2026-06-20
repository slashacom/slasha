import { useState } from 'react';
import { useNavigate } from 'react-router';
import {
  AlertTriangle,
  ArrowLeftIcon,
  ChevronDownIcon,
  ChevronUpIcon,
} from 'lucide-react';
import { isRouteErrorResponse } from 'react-router';

type ErrorViewProps = {
  error: unknown;
};

export function ErrorView(props: ErrorViewProps) {
  const { error } = props;
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
        <div className="mb-5 flex size-12 items-center justify-center rounded-full border border-red-500/20 bg-red-500/10">
          <AlertTriangle className="size-5 text-red-400" />
        </div>

        <h1 className="mb-1.5 text-[15px] font-medium tracking-tight text-text">
          {title}
        </h1>
        <p className="text-[13px] leading-relaxed text-text-secondary">
          {message}
        </p>

        {stack && (
          <div className="mt-4 w-full">
            <button
              onClick={() => setShowStack(!showStack)}
              className="mx-auto flex cursor-pointer items-center gap-1 font-mono text-[11px] uppercase tracking-[0.14em] text-text-tertiary transition-colors hover:text-text-secondary"
            >
              {showStack ? (
                <ChevronUpIcon className="size-3" />
              ) : (
                <ChevronDownIcon className="size-3" />
              )}
              {showStack ? 'Hide details' : 'Show details'}
            </button>
            {showStack && (
              <pre className="mt-3 max-h-48 overflow-auto rounded-md border border-border bg-code-bg p-3 text-left font-mono text-[11px] leading-relaxed text-code-text">
                {stack}
              </pre>
            )}
          </div>
        )}

        <button
          className="mt-6 flex h-9 cursor-pointer items-center gap-1.5 rounded-md border border-border bg-surface px-4 text-[12.5px] font-medium text-text-secondary transition-colors hover:bg-white/5 hover:text-text"
          onClick={() => navigate(-1)}
        >
          <ArrowLeftIcon className="size-3.5" />
          Go back
        </button>
      </div>
    </div>
  );
}
