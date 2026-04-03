import { QueryCache, QueryClient } from '@tanstack/react-query';

export const queryClient = new QueryClient({
  queryCache: new QueryCache({}),
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 1,
      retry: false,
    },
  },
});
