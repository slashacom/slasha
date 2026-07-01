import { useState, useEffect } from 'react';
import { ExternalLink, Check, X } from 'lucide-react';
import { toast } from 'sonner';
import { Github } from '~/components/icons/github';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import type { App } from '~/models/app';
import type { GithubAppConnection } from '~/queries/apps';
import {
  getGithubRepositoriesOptions,
  getGithubStatusOptions,
  useInstallGithub,
  useGetGithubBranchesQuery,
  useUpdateConnectionBranch,
} from '~/queries/connections';
import { useDisconnectGithub, useReconnectGithub } from '~/queries/apps';
import { queryClient } from '~/utils/query-client';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { RepositorySelect } from './repository-select';
import { BranchSelect } from './branch-select';

interface Props {
  app: App;
  connection?: GithubAppConnection;
}

export function GithubConnectionManager({ app, connection }: Props) {
  const [isEditing, setIsEditing] = useState(false);
  const [selectedRepository, setSelectedRepository] = useState<string>('');
  const [isDisconnectDialogOpen, setIsDisconnectDialogOpen] = useState(false);

  const [isEditingBranch, setIsEditingBranch] = useState(false);
  const [branchValue, setBranchValue] = useState(app.default_branch);

  const { data: githubStatus } = useSuspenseQuery(getGithubStatusOptions());
  const { data: reposData, isLoading: reposLoading } = useQuery({
    ...getGithubRepositoriesOptions(),
    enabled: isEditing && githubStatus?.enabled === true,
  });

  const installGithub = useInstallGithub();
  const reconnectGithub = useReconnectGithub();
  const disconnectGithub = useDisconnectGithub();
  const updateBranch = useUpdateConnectionBranch(app.slug);

  const { data: githubBranches, isFetching: branchesLoading } =
    useGetGithubBranchesQuery(
      connection?.installation_id,
      connection?.repository_id
    );

  useEffect(() => {
    setBranchValue(app.default_branch);
  }, [app.default_branch]);

  if (app.source !== 'github') {
    return null;
  }

  const repository = connection?.repository;
  const isConnected = repository != null;

  const handleConnectGithub = async () => {
    try {
      const data = await installGithub.mutateAsync({
        redirect_to: window.location.pathname,
      });
      window.location.href = data.url;
    } catch (err: any) {
      toast.error(err.message || 'Failed to connect to GitHub');
    }
  };

  const handleReconnect = async () => {
    if (!selectedRepository) {
      toast.error('Please select a repository');
      return;
    }
    const repositories = reposData?.repositories || [];
    const selectedRepoObj = repositories.find(
      (r) => r.id.toString() === selectedRepository
    );
    if (!selectedRepoObj) {
      toast.error('The selected repository is no longer available');
      return;
    }

    const promise = reconnectGithub.mutateAsync({
      appSlug: app.slug,
      installation_id: selectedRepoObj.installation_id,
      repository_id: selectedRepoObj.id,
    });

    toast.promise(promise, {
      loading: 'Connecting repository...',
      success: 'Repository connected successfully',
      error: 'Failed to connect repository',
    });

    try {
      await promise;
      await queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
      setIsEditing(false);
    } catch {}
  };

  const handleDisconnect = async () => {
    const promise = disconnectGithub.mutateAsync(app.slug);

    toast.promise(promise, {
      loading: 'Disconnecting repository...',
      success: 'Repository disconnected successfully',
      error: 'Failed to disconnect repository',
    });

    try {
      await promise;
      await queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
      setIsDisconnectDialogOpen(false);
    } catch {}
  };

  const handleSaveBranch = async () => {
    if (!branchValue.trim()) return;
    try {
      await updateBranch.mutateAsync(branchValue.trim());
      setIsEditingBranch(false);
      toast.success('Successfully updated default branch');
    } catch (err: any) {
      toast.error(err.message || 'Failed to update branch');
    }
  };

  const repositories = reposData?.repositories || [];

  return (
    <div>
      <h3 className="text-[14px] font-semibold text-text">GitHub Connection</h3>
      <p className="mt-1 text-[13px] text-text-tertiary">
        Manage the GitHub repository connected to this application.
      </p>

      <div className="mt-6 rounded-lg border border-border bg-surface p-6">
        {!isEditing ? (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Github className="size-5 text-text" />
              <div>
                {isConnected ? (
                  <>
                    <a
                      href={repository.html_url}
                      target="_blank"
                      rel="noreferrer"
                      className="flex items-center gap-1.5 text-[13px] font-medium text-text hover:underline"
                    >
                      {repository.full_name}
                      <ExternalLink className="size-3.5 text-text-tertiary" />
                    </a>
                    <div className="text-[12px] text-text-secondary mt-1">
                      <div className="flex items-center gap-2">
                        <span>Branch:</span>
                        {isEditingBranch ? (
                          <div className="flex items-center gap-2">
                            <div className="w-48">
                              {githubBranches || branchesLoading ? (
                                <BranchSelect
                                  branches={githubBranches?.branches || []}
                                  value={branchValue}
                                  onChange={setBranchValue}
                                  isLoading={branchesLoading}
                                />
                              ) : (
                                <Input
                                  value={branchValue}
                                  onChange={(e) =>
                                    setBranchValue(e.target.value)
                                  }
                                  className="h-7 text-[12px]"
                                />
                              )}
                            </div>
                            <Button
                              color="primary"
                              size="sm"
                              variant="ghost"
                              icon={<Check className="size-3.5" />}
                              onClick={handleSaveBranch}
                              isLoading={updateBranch.isPending}
                            />
                            <Button
                              color="neutral"
                              size="sm"
                              variant="ghost"
                              icon={<X className="size-3.5" />}
                              onClick={() => {
                                setIsEditingBranch(false);
                                setBranchValue(app.default_branch);
                              }}
                              isDisabled={updateBranch.isPending}
                            />
                          </div>
                        ) : (
                          <>
                            <span className="font-mono text-text font-medium">
                              {app.default_branch}
                            </span>
                            <button
                              type="button"
                              onClick={() => setIsEditingBranch(true)}
                              className="ml-1 text-[11px] font-medium text-text-secondary hover:text-text hover:underline"
                            >
                              Edit
                            </button>
                          </>
                        )}
                      </div>
                    </div>
                  </>
                ) : (
                  <p className="text-[13px] font-medium text-amber-500">
                    Not connected or repository not found
                  </p>
                )}
              </div>
            </div>
            <div className="flex gap-2">
              {isConnected && (
                <Button
                  variant="ghost"
                  label="Disconnect"
                  onClick={() => setIsDisconnectDialogOpen(true)}
                  isDisabled={disconnectGithub.isPending}
                  className="text-red-500 hover:text-red-500"
                />
              )}
              <Button
                color="neutral"
                label={isConnected ? 'Change Repository' : 'Connect Repository'}
                onClick={() => setIsEditing(true)}
              />
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            {reposLoading ? (
              <p className="text-sm text-text-tertiary">
                Loading repositories...
              </p>
            ) : repositories.length === 0 ? (
              <div className="text-center py-4">
                <p className="mb-4 text-sm text-text-secondary">
                  You haven't connected any GitHub accounts yet.
                </p>
                <Button
                  type="button"
                  color="neutral"
                  label="Connect GitHub Account"
                  onClick={handleConnectGithub}
                  isLoading={installGithub.isPending}
                />
              </div>
            ) : (
              <div className="space-y-4">
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <Label className="text-[12px] font-medium text-text-tertiary">
                      Repository
                    </Label>
                    <button
                      type="button"
                      onClick={handleConnectGithub}
                      className="text-[12px] font-medium text-text-secondary hover:text-text hover:underline"
                      disabled={installGithub.isPending}
                    >
                      Connect another account
                    </button>
                  </div>
                  <RepositorySelect
                    repositories={repositories}
                    value={selectedRepository}
                    onChange={setSelectedRepository}
                  />
                </div>

                <div className="flex justify-end gap-2 pt-2">
                  <Button
                    variant="ghost"
                    label="Cancel"
                    onClick={() => setIsEditing(false)}
                    isDisabled={reconnectGithub.isPending}
                  />
                  <Button
                    label="Save Connection"
                    onClick={handleReconnect}
                    isLoading={reconnectGithub.isPending}
                    isDisabled={
                      !selectedRepository || reconnectGithub.isPending
                    }
                  />
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      <ConfirmationDialog
        open={isDisconnectDialogOpen}
        onOpenChange={setIsDisconnectDialogOpen}
        title="Disconnect Repository"
        description="Are you sure you want to disconnect this repository?"
        confirmLabel="Disconnect"
        onConfirm={handleDisconnect}
      />
    </div>
  );
}
