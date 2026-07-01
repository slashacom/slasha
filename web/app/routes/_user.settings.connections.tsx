import { useSuspenseQuery } from '@tanstack/react-query';
import { Trash2Icon, PencilIcon } from 'lucide-react';
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
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import {
  getGithubRepositoriesOptions,
  getGithubStatusOptions,
  useInstallGithub,
  useRemoveGithubInstallation,
  getGithubSetupStatusOptions,
  useBeginGithubSetup,
  useUpdateGithubCredentials,
  useDeleteGithubSetup,
} from '~/queries/connections';
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
  const [disconnectId, setDisconnectId] = useState<number | null>(null);

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

  const handleDisconnect = async () => {
    if (disconnectId === null) return;
    const promise = removeInstallation.mutateAsync(disconnectId);
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
      setDisconnectId(null);
    } catch {}
  };

  return (
    <div className="rounded-lg border border-border bg-surface overflow-hidden">
      <div className="flex items-center justify-between p-5 border-b border-border">
        <div className="flex items-center gap-3">
          <Github className="size-6 text-text" />
          <div>
            <h4 className="text-sm font-medium text-text">
              GitHub Installations
            </h4>
            <p className="text-[13px] text-text-secondary mt-0.5">
              Connect to deploy applications from your GitHub repositories
            </p>
          </div>
        </div>
        {data.installations.length > 0 && (
          <Button
            size="sm"
            color="neutral"
            label="Connect Another"
            onClick={handleConnect}
            isLoading={installGithub.isPending}
          />
        )}
      </div>

      <div className="p-0">
        {data.installations.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-10 px-6 text-center">
            <p className="text-[13px] text-text-secondary mb-4">
              No GitHub accounts connected yet.
            </p>
            <Button
              color="neutral"
              label="Connect GitHub Account"
              onClick={handleConnect}
              isLoading={installGithub.isPending}
            />
          </div>
        ) : (
          <div className="divide-y divide-border">
            {data.installations.map((installation) => (
              <div
                key={installation.installation_id}
                className="flex items-center justify-between p-5 hover:bg-surface-hover transition-colors"
              >
                <div>
                  <p className="text-sm font-medium text-text">
                    GitHub Installation #{installation.installation_id}
                  </p>
                  <p className="text-[13px] text-text-secondary mt-1">
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
                    className="text-[12px] text-text-tertiary hover:text-text mt-2 inline-block hover:underline"
                  >
                    Configure on GitHub
                  </a>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  icon={<Trash2Icon className="size-4" />}
                  onClick={() => setDisconnectId(installation.installation_id)}
                  className="text-red-500/80 hover:text-red-500 hover:bg-red-500/10"
                />
              </div>
            ))}
          </div>
        )}
      </div>

      <ConfirmationDialog
        open={disconnectId !== null}
        onOpenChange={(open) => !open && setDisconnectId(null)}
        title="Disconnect GitHub Installation"
        description="Are you sure you want to disconnect this GitHub installation? This may affect apps using it."
        confirmLabel="Disconnect"
        onConfirm={handleDisconnect}
      />
    </div>
  );
}

function GithubAppSetupManager() {
  const { data: setupStatus } = useSuspenseQuery(getGithubSetupStatusOptions());
  const beginSetup = useBeginGithubSetup();
  const updateCredentials = useUpdateGithubCredentials();
  const deleteSetup = useDeleteGithubSetup();
  const [isEditing, setIsEditing] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);

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
    const promise = deleteSetup.mutateAsync();
    toast.promise(promise, {
      loading: 'Deleting setup...',
      success: () => {
        setIsDeleteDialogOpen(false);
        queryClient.invalidateQueries({
          queryKey: ['github', 'app-setup', 'status'],
        });
        queryClient.invalidateQueries({ queryKey: ['github', 'status'] });
        return 'GitHub App configuration deleted';
      },
      error: 'Failed to delete setup',
    });
  };

  const dialogs = (
    <>
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

      <ConfirmationDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
        title="Delete GitHub App Setup"
        description="Are you sure you want to delete the GitHub App configuration?"
        confirmLabel="Delete"
        onConfirm={handleDelete}
      />
    </>
  );

  if (!setupStatus.configured) {
    return (
      <div className="rounded-lg border border-border bg-surface p-8 text-center">
        <Github className="size-8 text-text-tertiary mx-auto mb-3" />
        <p className="text-sm font-medium text-text">
          GitHub Integration Disabled
        </p>
        <p className="text-[13px] text-text-secondary mt-1 mb-6">
          No GitHub App configured. Start the auto-setup or configure manually.
        </p>
        <div className="flex items-center justify-center gap-3">
          <Button
            variant="ghost"
            label="Manual Edit"
            onClick={() => setIsEditing(true)}
            className="whitespace-nowrap"
          />
          <Button
            color="neutral"
            label="Auto Setup"
            onClick={handleBeginSetup}
            isLoading={beginSetup.isPending}
            className="whitespace-nowrap"
          />
        </div>
        {dialogs}
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border bg-surface overflow-hidden">
      <div className="p-5 border-b border-border">
        <h4 className="text-sm font-medium text-text">
          GitHub App Configuration
        </h4>
        <p className="text-[13px] text-text-secondary mt-0.5">
          Configure your GitHub App
        </p>
      </div>

      <div className="p-5">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-text">
              App Configured (ID: {setupStatus.app_id})
            </p>
            <p className="text-[13px] text-text-secondary mt-1">
              Configured on{' '}
              {new Date(setupStatus.created_at!).toLocaleDateString()}
            </p>
          </div>
          <div className="flex items-center gap-1">
            <Button
              size="sm"
              variant="ghost"
              icon={<PencilIcon className="size-4" />}
              onClick={() => setIsEditing(true)}
            />
            <Button
              size="sm"
              variant="ghost"
              color="error"
              onClick={() => setIsDeleteDialogOpen(true)}
              isLoading={deleteSetup.isPending}
              icon={<Trash2Icon className="size-4" />}
            />
          </div>
        </div>
      </div>
      {dialogs}
    </div>
  );
}
export default function ConnectionsSettings() {
  const { data: status } = useSuspenseQuery(getGithubStatusOptions());
  const { data: authMe } = useSuspenseQuery(getAuthMeOptions());

  return (
    <div className="space-y-6 max-w-2xl">
      <div>
        <h3 className="font-semibold text-text">Connected Accounts</h3>
        <p className="mt-2 text-[13px] text-text-secondary">
          Manage integrations with external services like GitHub.
        </p>
      </div>

      {status.enabled ? (
        <EnabledGithubConnections />
      ) : (
        authMe.user.role !== 'Admin' && (
          <div className="rounded-lg border border-border bg-surface p-6 text-center">
            <Github className="size-8 text-text-tertiary mx-auto mb-3" />
            <p className="text-sm font-medium text-text">
              GitHub Integration Disabled
            </p>
            <p className="text-[13px] text-text-secondary mt-1">
              GitHub integration is not enabled on this Slasha instance.
            </p>
          </div>
        )
      )}

      {authMe.user.role === 'Admin' && <GithubAppSetupManager />}
    </div>
  );
}
