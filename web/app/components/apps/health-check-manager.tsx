import { useState } from 'react';
import { useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { HeartPulse } from 'lucide-react';
import { toast } from 'sonner';
import { getAppEnvVarsOptions, useUpdateAppEnvVars } from '~/queries/apps';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { HStack, VStack } from '~/components/interface/stacks';

const HEALTH_CHECK_PATH_KEY = 'SLASHA_HEALTH_CHECK_PATH';
const HEALTH_CHECK_TIMEOUT_KEY = 'SLASHA_HEALTH_CHECK_TIMEOUT';

type HealthCheckManagerProps = {
  appSlug: string;
};

export function HealthCheckManager(props: HealthCheckManagerProps) {
  const { appSlug } = props;
  const queryClient = useQueryClient();
  const updateEnvVars = useUpdateAppEnvVars();
  const { data: envData } = useSuspenseQuery(getAppEnvVarsOptions(appSlug));

  const savedPath = envData.env_vars[HEALTH_CHECK_PATH_KEY] ?? '';
  const savedTimeout = envData.env_vars[HEALTH_CHECK_TIMEOUT_KEY] ?? '';

  const [path, setPath] = useState(savedPath);
  const [timeoutSecs, setTimeoutSecs] = useState(savedTimeout);

  const normalizedPath =
    path.trim() && !path.trim().startsWith('/')
      ? `/${path.trim()}`
      : path.trim();
  const trimmedTimeout = timeoutSecs.trim();
  const isDirty =
    normalizedPath !== savedPath || trimmedTimeout !== savedTimeout;

  const handleSave = async () => {
    if (!isDirty) {
      return;
    }

    if (trimmedTimeout && !/^[1-9]\d*$/.test(trimmedTimeout)) {
      toast.error('Timeout must be a positive number of seconds');
      return;
    }

    const vars = { ...envData.env_vars };

    if (normalizedPath) {
      vars[HEALTH_CHECK_PATH_KEY] = normalizedPath;
    } else {
      delete vars[HEALTH_CHECK_PATH_KEY];
    }

    if (trimmedTimeout) {
      vars[HEALTH_CHECK_TIMEOUT_KEY] = trimmedTimeout;
    } else {
      delete vars[HEALTH_CHECK_TIMEOUT_KEY];
    }

    const promise = updateEnvVars.mutateAsync({ appSlug, vars });

    toast.promise(promise, {
      loading: 'Saving health check settings...',
      success: () => {
        queryClient.invalidateQueries({
          queryKey: ['apps', appSlug, 'env-vars'],
        });
        setPath(normalizedPath);
        return 'Health check settings saved';
      },
      error: (error) =>
        error.message || 'Failed to save health check settings.',
    });
  };

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div className="px-6 py-5">
          <HStack space={3} alignItems="start">
            <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
              <HeartPulse className="size-5" />
            </div>
            <div className="min-w-0 flex-1">
              <h3 className="text-[15px] font-semibold text-text">
                Health Check
              </h3>
              <p className="mt-0.5 text-[13px] text-text-tertiary">
                Before traffic switches to a new deployment, Slasha probes the
                web process over HTTP until it responds. A release that never
                becomes ready is rolled back while the previous deployment keeps
                serving.
              </p>

              <div className="mt-4 flex flex-wrap items-end gap-3">
                <VStack space={1.5}>
                  <Label
                    htmlFor="health-check-path"
                    className="text-[12px] font-medium text-text-secondary"
                  >
                    Path
                  </Label>
                  <Input
                    id="health-check-path"
                    value={path}
                    onChange={(event) => setPath(event.target.value)}
                    placeholder="/"
                    className="w-56 font-mono"
                  />
                </VStack>
                <VStack space={1.5}>
                  <Label
                    htmlFor="health-check-timeout"
                    className="text-[12px] font-medium text-text-secondary"
                  >
                    Timeout (seconds)
                  </Label>
                  <Input
                    id="health-check-timeout"
                    value={timeoutSecs}
                    onChange={(event) => setTimeoutSecs(event.target.value)}
                    placeholder="60"
                    inputMode="numeric"
                    className="w-36"
                  />
                </VStack>
                <Button
                  label="Save"
                  size="sm"
                  className="mb-0.5"
                  onClick={handleSave}
                  disabled={updateEnvVars.isPending || !isDirty}
                />
              </div>

              <p className="mt-3 text-[12px] text-text-tertiary">
                With no path set, any response below 500 on{' '}
                <span className="font-mono text-text-secondary">/</span> counts
                as ready within 60 seconds. A configured path must respond with
                2xx or 3xx. Changes apply from the next deployment.
              </p>
            </div>
          </HStack>
        </div>
      </div>
    </VStack>
  );
}
