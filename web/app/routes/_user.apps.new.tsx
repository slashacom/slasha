import { useState } from 'react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { GitBranch, Link as LinkIcon } from 'lucide-react';
import { Github } from '~/components/icons/github';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { getCheckSlugOptions, useCreateApp } from '~/queries/apps';
import { RepositorySelect } from '~/components/apps/repository-select';
import {
  getGithubRepositoriesOptions,
  getGithubStatusOptions,
  useInstallGithub,
} from '~/queries/github-app';
import { queryClient } from '~/utils/query-client';
import { useDebounce } from '~/hooks/use-debounce';
import type { AppSource } from '~/models/app';

export function meta() {
  return [{ title: 'New app · slasha' }];
}

export async function clientLoader() {
  await queryClient.ensureQueryData(getGithubStatusOptions());
}

export default function NewApp() {
  const navigate = useNavigate();
  const createApp = useCreateApp();
  const installGithub = useInstallGithub();

  const [name, setName] = useState('');
  const [source, setSource] = useState<AppSource>('local');
  const [selectedRepository, setSelectedRepository] = useState<string>('');
  const [gitUrl, setGitUrl] = useState('');
  const [gitBranch, setGitBranch] = useState('');

  const debouncedName = useDebounce(name, 300);

  const { data: slugCheck, isFetching: slugChecking } = useQuery({
    ...getCheckSlugOptions(debouncedName),
    enabled: debouncedName.trim().length > 0,
  });

  const { data: githubStatus } = useSuspenseQuery(getGithubStatusOptions());
  const { data: reposData, isLoading: reposLoading } = useQuery({
    ...getGithubRepositoriesOptions(),
    enabled: githubStatus?.enabled === true && source === 'github',
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (source === 'github' && !selectedRepository) {
      toast.error('Please select a GitHub repository');
      return;
    }

    const repositories = reposData?.repositories || [];
    const selectedRepoObj = repositories.find(
      (r) => r.id.toString() === selectedRepository
    );

    if (source === 'github' && !selectedRepoObj) {
      toast.error('The selected GitHub repository is no longer available');
      return;
    }

    const payload =
      source === 'github'
        ? {
            name,
            source,
            installation_id: selectedRepoObj!.installation_id,
            repository_id: selectedRepoObj!.id,
          }
        : source === 'git'
          ? {
              name,
              source,
              url: gitUrl.trim(),
              ...(gitBranch.trim() ? { branch: gitBranch.trim() } : {}),
            }
          : { name, source };
    const promise = createApp.mutateAsync(payload);

    toast.promise(promise, {
      loading: 'Creating application...',
      success: `Successfully created ${name}`,
      error: (err) => err.message || 'Failed to create application.',
    });

    try {
      const data = await promise;
      void queryClient.invalidateQueries({ queryKey: ['apps'] });
      navigate(`/apps/${data.app.slug}`);
    } catch {}
  };

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

  const installations = reposData?.installations || [];
  const repositories = reposData?.repositories || [];

  return (
    <div>
      <div>
        <h3 className="font-semibold text-text">New app</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Give your application a name and choose how to deploy it.
        </p>
      </div>

      <div className="mt-6">
        <form onSubmit={handleSubmit} className="w-full max-w-md space-y-6">
          <div className="space-y-1.5">
            <Label
              htmlFor="name"
              className="text-[13px] font-medium text-text-secondary"
            >
              Application name
            </Label>
            <Input
              id="name"
              name="name"
              type="text"
              required
              placeholder="my-awesome-app"
              autoFocus
              className="h-10"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <div className="h-5">
              {createApp.isPending ? null : name.trim() === '' ? (
                <p className="text-xs text-text-tertiary">
                  Used to generate the slug and git repository name.
                </p>
              ) : slugChecking || debouncedName !== name ? (
                <p className="text-xs text-text-tertiary animate-pulse">
                  Checking availability...
                </p>
              ) : slugCheck ? (
                <p className="text-xs text-text-tertiary">
                  URL:{' '}
                  <span className="font-mono text-text-secondary">
                    {slugCheck.slug}
                  </span>
                  {!slugCheck.available && (
                    <span className="ml-2 text-amber-500/90">
                      (Name taken, using suggested unique name)
                    </span>
                  )}
                </p>
              ) : null}
            </div>
          </div>

          <div className="space-y-4 pt-2">
            <Label className="text-[13px] font-medium text-text-secondary">
              Deployment Source
            </Label>
            <div className="grid grid-cols-3 gap-2">
              <Button
                color={source === 'local' ? 'primary' : 'neutral'}
                label="Slasha Git"
                type="button"
                onClick={() => setSource('local')}
              />
              <Button
                color={source === 'github' ? 'primary' : 'neutral'}
                label="GitHub"
                type="button"
                icon={<Github className="size-4" />}
                onClick={() => setSource('github')}
                isDisabled={!githubStatus?.enabled}
              />
              <Button
                color={source === 'git' ? 'primary' : 'neutral'}
                label="Git URL"
                type="button"
                icon={<GitBranch className="size-4" />}
                onClick={() => setSource('git')}
              />
            </div>
          </div>

          {source === 'github' && (
            <div className="space-y-4 rounded-lg border border-border bg-surface p-4">
              {reposLoading ? (
                <p className="text-sm text-text-tertiary">
                  Loading repositories...
                </p>
              ) : installations.length === 0 ? (
                <div className="text-center">
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
              ) : repositories.length === 0 ? (
                <div className="text-center py-4">
                  <p className="mb-4 text-sm text-text-secondary">
                    No repositories found in your connected installations.
                  </p>
                  <button
                    type="button"
                    onClick={handleConnectGithub}
                    className="text-sm font-medium text-text-secondary hover:text-text hover:underline"
                    disabled={installGithub.isPending}
                  >
                    Connect another account
                  </button>
                </div>
              ) : (
                <>
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
                </>
              )}
            </div>
          )}

          {source === 'local' && (
            <div className="rounded-lg border border-border bg-surface p-4">
              <p className="text-sm text-text-secondary leading-relaxed">
                Slasha will host a Git repository for this app. Push to it over
                HTTPS or SSH with Git to trigger deployments.
              </p>
            </div>
          )}

          {source === 'git' && (
            <div className="space-y-4 rounded-lg border border-border bg-surface p-4">
              <div className="space-y-1.5">
                <Label
                  htmlFor="git-url"
                  className="text-[12px] font-medium text-text-tertiary"
                >
                  Repository URL
                </Label>
                <div className="relative">
                  <LinkIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
                  <Input
                    id="git-url"
                    type="url"
                    required
                    value={gitUrl}
                    onChange={(event) => setGitUrl(event.target.value)}
                    placeholder="https://example.com/team/repository.git"
                    className="h-10 pl-9 font-mono text-[13px]"
                  />
                </div>
                <p className="text-xs text-text-tertiary">
                  Public HTTP(S) repositories only.
                </p>
              </div>
              <div className="space-y-1.5">
                <Label
                  htmlFor="git-branch"
                  className="text-[12px] font-medium text-text-tertiary"
                >
                  Branch
                </Label>
                <div className="relative">
                  <GitBranch className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
                  <Input
                    id="git-branch"
                    value={gitBranch}
                    onChange={(event) => setGitBranch(event.target.value)}
                    placeholder="Remote default branch"
                    className="h-10 pl-9 font-mono text-[13px]"
                  />
                </div>
              </div>
            </div>
          )}

          <div className="flex items-center justify-end gap-2 pt-4">
            <Button
              variant="ghost"
              label="Cancel"
              type="button"
              onClick={() => navigate('/apps')}
              isDisabled={createApp.isPending}
            />
            <Button
              type="submit"
              label="Create app"
              isLoading={createApp.isPending}
              isDisabled={
                createApp.isPending ||
                debouncedName !== name ||
                slugChecking ||
                (source === 'github' && !selectedRepository) ||
                (source === 'git' && !gitUrl.trim())
              }
            />
          </div>
        </form>
      </div>
    </div>
  );
}
