import {
  queryOptions,
  useMutation,
  useQueryClient,
} from '@tanstack/react-query';
import { httpGet, httpPut } from '~/utils/http';
import type { ServerSettings } from '~/models/server_settings';

export function getServerSettingsOptions() {
  return queryOptions({
    queryKey: ['server-settings'],
    queryFn: () => httpGet<ServerSettings>('server-settings'),
  });
}

export function useUpdateServerSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (payload: ServerSettings) =>
      httpPut<ServerSettings>('server-settings', payload),
    onSuccess: (data) => {
      queryClient.setQueryData(['server-settings'], data);
    },
  });
}
