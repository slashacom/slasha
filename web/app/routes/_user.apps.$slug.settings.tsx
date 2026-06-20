import { useState } from 'react';
import { useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import {
  getAppDomainsOptions,
  getAppEnvSuggestionsOptions,
  getAppEnvVarsOptions,
  getAppOptions,
  useDeleteApp,
} from '~/queries/apps';
import { AppEnvEditor } from '~/components/apps/env-editor';
import { DomainManager } from '~/components/apps/domain-manager';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(getAppOptions(params.slug)),
    queryClient.ensureQueryData(getAppEnvVarsOptions(params.slug)),
    queryClient.ensureQueryData(getAppEnvSuggestionsOptions(params.slug)),
    queryClient.ensureQueryData(getAppDomainsOptions(params.slug)),
  ]);
}

export default function AppSettingsPage() {
  const { slug } = useParams();
  const navigate = useNavigate();
  const deleteApp = useDeleteApp();
  const { data } = useSuspenseQuery(getAppOptions(slug!));
  const app = data.app;
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  if (!app) {
    return null;
  }

  return (
    <div className="flex-1 overflow-y-auto p-8">
      <div className="max-w-3xl mb-8">
        <AppEnvEditor appSlug={slug!} />
      </div>
      <div className="max-w-3xl mb-12">
        <DomainManager appSlug={slug!} />
      </div>
      <div className="max-w-2xl">
        <h3 className="text-[14px] font-semibold text-text">Danger Zone</h3>
        <p className="mt-1 text-[13px] text-text-tertiary">
          Destructive actions for your application.
        </p>

        <div className="mt-6 rounded-lg border border-red-500/20 bg-red-500/5 p-6">
          <div className="flex items-start justify-between gap-6">
            <div>
              <h4 className="text-[13px] font-medium text-red-500">
                Delete this application
              </h4>
              <p className="mt-1 text-[12px] text-red-500/70">
                Once you delete an application, there is no going back. Please be
                certain.
              </p>
            </div>
            <button
              onClick={() => setShowDeleteConfirm(true)}
              className="shrink-0 rounded-md bg-red-600 px-3 py-1.5 text-[12px] font-medium text-white transition-colors hover:bg-red-500"
            >
              Delete App
            </button>
          </div>
        </div>
      </div>

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Application"
        description={`Are you sure you want to delete ${app.name}? This action cannot be undone and will permanently delete all associated data.`}
        confirmLabel="Delete Application"
        onConfirm={() => {
          deleteApp.mutate(app.slug, {
            onSuccess: () => {
              navigate('/apps');
            },
          });
        }}
      />
    </div>
  );
}
