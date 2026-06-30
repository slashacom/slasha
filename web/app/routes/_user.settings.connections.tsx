import { useSuspenseQuery } from '@tanstack/react-query';
import { Trash2Icon } from 'lucide-react';
import { toast } from 'sonner';
import { Github } from '~/components/icons/github';
import { Button } from '~/components/interface/button';
import { VStack } from '~/components/interface/stacks';
import {
  getGithubRepositoriesOptions,
  getGithubStatusOptions,
  useInstallGithub,
  useRemoveGithubInstallation,
} from '~/queries/github';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'Connections · slasha' }];
}

export async function clientLoader() {
  const status = await queryClient.ensureQueryData(getGithubStatusOptions());
  if (status.enabled) {
    await queryClient.ensureQueryData(getGithubRepositoriesOptions());
  }
}

function EnabledGithubConnections() {
  const { data } = useSuspenseQuery(getGithubRepositoriesOptions());
  const installGithub = useInstallGithub();
  const removeInstallation = useRemoveGithubInstallation();

  const handleConnect = async () => {
    try {
      const response = await installGithub.mutateAsync({
        redirect_to: window.location.pathname,
      });
      window.location.href = response.url;
    } catch (error: any) {
      toast.error(error.message || 'Failed to connect to GitHub');
    }
  };

  const handleDisconnect = async (installationId: number) => {
    if (
      !confirm(
        'Are you sure you want to disconnect this GitHub installation? This may affect apps using it.'
      )
    ) {
      return;
    }

    const promise = removeInstallation.mutateAsync(installationId);
    toast.promise(promise, {
      loading: 'Disconnecting...',
      success: 'GitHub installation disconnected',
      error: 'Failed to disconnect GitHub',
    });
    try {
      await promise;
      await queryClient.invalidateQueries({
        queryKey: ['github', 'repositories'],
      });
    } catch {}
  };

  return (
    <VStack space={4}>
      <div className="flex items-center justify-between rounded-lg border border-border bg-surface p-4">
        <div className="flex items-center gap-3">
          <Github className="size-5 text-text" />
          <div>
            <p className="text-sm font-medium text-text">GitHub</p>
            <p className="text-xs text-text-secondary">
              Connect to deploy from GitHub repositories
            </p>
          </div>
        </div>
        <Button
          color="neutral"
          label="Connect"
          onClick={handleConnect}
          isLoading={installGithub.isPending}
        />
      </div>

      {data.installations.length > 0 && (
        <div className="space-y-4 pt-4">
          <h4 className="text-sm font-medium text-text">
            Connected Installations
          </h4>
          {data.installations.map((installation) => (
            <div
              key={installation.installation_id}
              className="flex items-center justify-between rounded-lg border border-border p-4"
            >
              <div>
                <p className="text-sm font-medium text-text">
                  GitHub Installation #{installation.installation_id}
                </p>
                <p className="text-xs text-text-secondary mt-1">
                  {installation.repositories_count}{' '}
                  {installation.repositories_count === 1
                    ? 'repository'
                    : 'repositories'}{' '}
                  connected
                </p>
                <a
                  href={installation.configure_url}
                  target="_blank"
                  rel="noreferrer"
                  className="text-xs text-text-tertiary hover:text-text mt-2 block hover:underline"
                >
                  Configure on GitHub
                </a>
              </div>
              <Button
                variant="ghost"
                size="sm"
                icon={<Trash2Icon className="size-4" />}
                onClick={() => handleDisconnect(installation.installation_id)}
                className="text-red-500/80 hover:text-red-500 hover:bg-red-500/10"
              />
            </div>
          ))}
        </div>
      )}
    </VStack>
  );
}

export default function ConnectionsSettings() {
  const { data: status } = useSuspenseQuery(getGithubStatusOptions());

  return (
    <div className="space-y-6 max-w-xl">
      <div>
        <h3 className="font-semibold text-text">Connected Accounts</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Manage integrations with external services like GitHub.
        </p>
      </div>

      {status.enabled ? (
        <EnabledGithubConnections />
      ) : (
        <div className="rounded-lg border border-border bg-surface p-6">
          <p className="text-sm text-text-secondary">
            GitHub integration is not enabled on this Slasha instance.
          </p>
        </div>
      )}
    </div>
  );
}
