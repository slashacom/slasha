import { queryOptions } from '@tanstack/react-query';
import { httpGet } from '~/utils/http';

export type HealthResponse = {
  status: string;
  version: string;
};

export function getHealthOptions() {
  return queryOptions({
    queryKey: ['health'],
    queryFn: () => httpGet<HealthResponse>('health', undefined),
    staleTime: Infinity,
  });
}
