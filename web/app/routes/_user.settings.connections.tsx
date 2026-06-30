import { useSuspenseQuery } from '@tanstack/react-query';
import { Trash2Icon } from 'lucide-react';
import { toast } from 'sonner';
import { useState } from 'react';
import { Github } from '~/components/icons/github';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { Textarea } from '~/components/interface/textarea';
import { VStack } from '~/components/interface/stacks';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import {
  getGithubRepositoriesOptions,
  getGithubStatusOptions,
  useInstallGithub,
  useRemoveGithubInstallation,
  getGithubSetupStatusOptions,
  useBeginGithubSetup,
  useUpdateGithubCredentials,
  useDeleteGithubSetup,
} from '~/queries/github-app';
import { getAuthMeOptions } from '~/queries/auth';
import { queryClient } from '~/utils/query-client';

export function meta() {
  return [{ title: 'Connections · slasha' }];
}

export async function clientLoader() {
  const authMe = await queryClient.ensureQueryData(getAuthMeOptions());
  const status = await queryClient.ensureQueryData(getGithubStatusOptions());
  if (status.enabled) {
    await queryClient.ensureQueryData(getGithubRepositoriesOptions());
  }
  if (authMe.user.role === 'Admin') {
    await queryClient.ensureQueryData(getGithubSetupStatusOptions());
  }
  return null;
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

function GithubAppSetupManager() {
  const { data: setupStatus } = useSuspenseQuery(getGithubSetupStatusOptions());
  const beginSetup = useBeginGithubSetup();
  const updateCredentials = useUpdateGithubCredentials();
  const deleteSetup = useDeleteGithubSetup();
  const [isEditing, setIsEditing] = useState(false);

  const handleBeginSetup = async () => {
    try {
      const response = await beginSetup.mutateAsync();
      const form = document.createElement('form');
      form.method = 'POST';
      form.action = response.github_url;
      const manifestInput = document.createElement('input');
      manifestInput.type = 'hidden';
      manifestInput.name = 'manifest';
      manifestInput.value = response.manifest;
      form.appendChild(manifestInput);
      document.body.appendChild(form);
      form.submit();
    } catch (error: any) {
      toast.error(error.message || 'Failed to start GitHub App setup');
    }
  };

  const handleManualUpdate = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const promise = updateCredentials.mutateAsync({
      app_id: formData.get('app_id') as string,
      client_id: formData.get('client_id') as string,
      client_secret: formData.get('client_secret') as string,
      private_key: formData.get('private_key') as string,
      webhook_secret: formData.get('webhook_secret') as string,
    });
    toast.promise(promise, {
      loading: 'Saving credentials...',
      success: () => {
        setIsEditing(false);
        queryClient.invalidateQueries({
          queryKey: ['github', 'app-setup', 'status'],
        });
        queryClient.invalidateQueries({ queryKey: ['github', 'status'] });
        return 'GitHub App credentials updated';
      },
      error: 'Failed to update credentials',
    });
  };

  const handleDelete = async () => {
    if (
      !confirm(
        'Are you sure you want to delete the GitHub App configuration? This will break all GitHub integrations.'
      )
    ) {
      return;
    }
    const promise = deleteSetup.mutateAsync();
    toast.promise(promise, {
      loading: 'Deleting setup...',
      success: () => {
        queryClient.invalidateQueries({
          queryKey: ['github', 'app-setup', 'status'],
        });
        queryClient.invalidateQueries({ queryKey: ['github', 'status'] });
        return 'GitHub App configuration deleted';
      },
      error: 'Failed to delete setup',
    });
  };

  return (
    <VStack space={4} className="border-t border-border pt-6 mt-6">
      <div>
        <h4 className="font-medium text-text">Platform Setup: GitHub App</h4>
        <p className="text-sm text-text-secondary mt-1">
          Configure the GitHub App to enable GitHub integration for this Slasha
          instance.
        </p>
      </div>

      <div className="rounded-lg border border-border bg-surface p-4">
        {setupStatus.configured ? (
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-text">
                Configured (App ID: {setupStatus.app_id})
              </p>
              <p className="text-xs text-text-secondary mt-1">
                Configured on{' '}
                {new Date(setupStatus.created_at!).toLocaleDateString()}
              </p>
            </div>
            <div className="flex gap-2">
              <Button
                size="sm"
                color="neutral"
                label="Recreate"
                onClick={handleBeginSetup}
                isLoading={beginSetup.isPending}
              />
              <Button
                size="sm"
                color="neutral"
                label="Edit"
                onClick={() => setIsEditing(true)}
              />
              <Button
                size="sm"
                variant="ghost"
                className="text-red-500"
                onClick={handleDelete}
                isLoading={deleteSetup.isPending}
              />
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-between">
            <p className="text-sm text-text-secondary">
              No GitHub App configured.
            </p>
            <div className="flex gap-2">
              <Button
                size="sm"
                color="neutral"
                label="Manual Edit"
                onClick={() => setIsEditing(true)}
              />
              <Button
                size="sm"
                color="neutral"
                label="Auto Setup"
                onClick={handleBeginSetup}
                isLoading={beginSetup.isPending}
              />
            </div>
          </div>
        )}
      </div>

      <Dialog open={isEditing} onOpenChange={setIsEditing}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Manual GitHub App Setup</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleManualUpdate} className="space-y-4">
            <VStack space={2}>
              <Label htmlFor="app_id">App ID</Label>
              <Input
                id="app_id"
                name="app_id"
                required
                defaultValue={setupStatus.app_id || ''}
              />
            </VStack>
            <VStack space={2}>
              <Label htmlFor="client_id">Client ID</Label>
              <Input id="client_id" name="client_id" required />
            </VStack>
            <VStack space={2}>
              <Label htmlFor="client_secret">Client Secret</Label>
              <Input
                id="client_secret"
                name="client_secret"
                type="password"
                required
              />
            </VStack>
            <VStack space={2}>
              <Label htmlFor="webhook_secret">Webhook Secret</Label>
              <Input
                id="webhook_secret"
                name="webhook_secret"
                type="password"
                required
              />
            </VStack>
            <VStack space={2}>
              <Label htmlFor="private_key">Private Key (PEM)</Label>
              <Textarea
                id="private_key"
                name="private_key"
                rows={5}
                className="font-mono text-xs"
                required
              />
            </VStack>
            <div className="flex justify-end gap-2 pt-2">
              <Button
                variant="ghost"
                label="Cancel"
                onClick={() => setIsEditing(false)}
                type="button"
              />
              <Button
                color="neutral"
                label="Save Credentials"
                type="submit"
                isLoading={updateCredentials.isPending}
              />
            </div>
          </form>
        </DialogContent>
      </Dialog>
    </VStack>
  );
}

export default function ConnectionsSettings() {
  const { data: status } = useSuspenseQuery(getGithubStatusOptions());
  const { data: authMe } = useSuspenseQuery(getAuthMeOptions());

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

      {authMe.user.role === 'Admin' && <GithubAppSetupManager />}
    </div>
  );
}
