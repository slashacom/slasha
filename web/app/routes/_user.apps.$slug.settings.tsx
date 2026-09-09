import { useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import {
  getAppDomainsOptions,
  getAppConnectionOptions,
  getAppEnvSuggestionsOptions,
  getAppEnvVarsOptions,
  getAppOptions,
  useDeleteApp,
  getAppDirectoriesOptions,
} from '~/queries/apps';
import { getNodesOptions } from '~/queries/nodes';
import { AppEnvEditor } from '~/components/apps/app-env-editor';
import { AppNameManager } from '~/components/apps/app-name-manager';
import { AppRootDirManager } from '~/components/apps/app-root-dir-manager';
import { AppNodeManager } from '~/components/apps/app-node-manager';
import { AutoDeployManager } from '~/components/apps/auto-deploy-manager';
import { HealthCheckManager } from '~/components/apps/health-check-manager';
import { GithubConnectionManager } from '~/components/apps/github-connection-manager';
import { GitConnectionManager } from '~/components/apps/git-connection-manager';
import { BackupManager } from '~/components/apps/backup-manager';
import { DomainManager } from '~/components/apps/domain-manager';
import { StorageManager } from '~/components/apps/storage-manager';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { getBackupOptions, getVolumesOptions } from '~/queries/storage';
import { queryClient } from '~/utils/query-client';
import { getGithubStatusOptions } from '~/queries/connections';
import { DangerZone } from '~/components/global/danger-zone';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  // Only the two queries this route reads block the navigation; the section
  // cards below fetch their own data and suspend into the page skeleton.
  void queryClient.prefetchQuery(getGithubStatusOptions());
  void queryClient.prefetchQuery(getAppEnvVarsOptions(params.slug));
  void queryClient.prefetchQuery(getAppEnvSuggestionsOptions(params.slug));
  void queryClient.prefetchQuery(getAppDomainsOptions(params.slug));
  void queryClient.prefetchQuery(getVolumesOptions(params.slug));
  void queryClient.prefetchQuery(getBackupOptions(params.slug));
  void queryClient.prefetchQuery(getNodesOptions());
  void queryClient.prefetchQuery(getAppDirectoriesOptions(params.slug));

  await Promise.all([
    queryClient.ensureQueryData(getAppOptions(params.slug)),
    queryClient.ensureQueryData(getAppConnectionOptions(params.slug)),
  ]);
}

export default function AppSettingsPage() {
  const { slug } = useParams();
  const navigate = useNavigate();
  const deleteApp = useDeleteApp();
  const { data } = useSuspenseQuery(getAppOptions(slug!));
  const { data: connectionData } = useSuspenseQuery(
    getAppConnectionOptions(slug!)
  );
  const app = data.app;
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  if (!app) {
    return null;
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col">
      <div className="flex-1 overflow-y-auto px-8 py-6">
        <div className="max-w-3xl space-y-8">
          <AppNameManager app={app} />
          <AppRootDirManager app={app} />
          <AppNodeManager app={app} />
          <AutoDeployManager app={app} />
          <HealthCheckManager appSlug={slug!} />
          <AppEnvEditor appSlug={slug!} />
          {app.source === 'github' && (
            <GithubConnectionManager
              app={app}
              connection={
                connectionData.connection &&
                'repository' in connectionData.connection
                  ? connectionData.connection
                  : undefined
              }
            />
          )}
          {app.source === 'git' && (
            <GitConnectionManager
              app={app}
              connection={
                connectionData.connection &&
                'clone_url' in connectionData.connection
                  ? connectionData.connection
                  : undefined
              }
            />
          )}
          <DomainManager appSlug={slug!} />
          <StorageManager appSlug={slug!} />
          <BackupManager appSlug={slug!} />
          <DangerZone
            description="Destructive actions for your application."
            actionTitle="Delete this application"
            actionDescription="Once you delete an application, there is no going back. Please be certain."
            actionLabel="Delete App"
            onAction={() => setShowDeleteConfirm(true)}
          />
        </div>

        <ConfirmationDialog
          open={showDeleteConfirm}
          onOpenChange={setShowDeleteConfirm}
          title="Delete Application"
          description={`Deleting ${app.name} is permanent. Its containers, deployments, services and volumes go with it.`}
          confirmLabel="Delete application"
          confirmText={app.slug}
          isPending={deleteApp.isPending}
          onConfirm={() => {
            deleteApp.mutate(app.slug, {
              onSuccess: () => {
                queryClient.invalidateQueries({ queryKey: ['apps'] });
                navigate('/apps');
              },
            });
          }}
        />
      </div>
    </div>
  );
}
