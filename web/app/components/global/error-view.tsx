import { useState } from 'react';
import { useNavigate } from 'react-router';
import { ArrowLeftIcon, ChevronDownIcon, ChevronUpIcon } from 'lucide-react';
import { SadIcon } from '~/components/icons/sad';
import { isRouteErrorResponse } from 'react-router';
import { FetchError } from '~/utils/http';

interface ErrorViewProps {
  error: unknown;
}

export function ErrorView({ error }: ErrorViewProps) {
  let title = "Uh oh! That's not good.";
  let message = 'We hit a snag. Please try again later.';
  let stack: string | undefined;

  if (isRouteErrorResponse(error) || error instanceof FetchError) {
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
      message = (error as FetchError).message;
    }
  } else if (import.meta.env.DEV && error && error instanceof Error) {
    message = error.message;
    stack = error.stack;
  }

  const [showStack, setShowStack] = useState(false);
  const navigate = useNavigate();

  return (
    <div className="flex h-full w-full flex-col items-center justify-center bg-white">
      <SadIcon className="mb-4 size-20 text-gray-300" />
      <h1 className="text-2xl mb-2 font-medium text-black">{title}</h1>
      <p className="text-gray-800">{message}</p>
      {stack && (
        <div className="mt-2 flex flex-col items-center gap-2">
          <button
            onClick={() => setShowStack(!showStack)}
            className="flex cursor-pointer items-center gap-1 text-sm text-red-500 hover:text-red-400"
          >
            {showStack ? (
              <>
                <ChevronUpIcon className="size-4" />
                hide more info
              </>
            ) : (
              <>
                <ChevronDownIcon className="size-4" />
                show more info
              </>
            )}
          </button>
          {showStack && (
            <span className="max-w-lg overflow-x-auto text-red-600">
              {stack}
            </span>
          )}
        </div>
      )}

      <button
        className="mt-4 flex cursor-pointer flex-row items-center rounded-lg bg-gray-200 px-4 py-2 text-sm font-medium text-black hover:bg-neutral-300"
        onClick={() => navigate(-1)}
      >
        <ArrowLeftIcon className="mr-2 size-4" />
        Go back
      </button>
    </div>
  );
}
