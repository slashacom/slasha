import { useMemo } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import {
  getAppEnvSuggestionsOptions,
  getAppEnvVarsOptions,
  useUpdateAppEnvVars,
} from '~/queries/apps';
import {
  getDeploymentsOptions,
  useRollbackDeployment,
} from '~/queries/deployments';
import {
  APP_SLASHA_REFS,
  type SuggestionGroup,
} from '~/components/apps/env-dotenv-editor';
import { EnvEditor } from '~/components/apps/env-editor';

type AppEnvEditorProps = {
  appSlug: string;
};

export function AppEnvEditor(props: AppEnvEditorProps) {
  const { appSlug } = props;
  const queryClient = useQueryClient();
  const { data: envData, isLoading: envLoading } = useQuery(
    getAppEnvVarsOptions(appSlug)
  );
  const { data: suggestionsData } = useQuery(
    getAppEnvSuggestionsOptions(appSlug)
  );
  const updateEnvVars = useUpdateAppEnvVars();
  const { data: deploymentsData } = useQuery(getDeploymentsOptions(appSlug));
  const rollbackDeployment = useRollbackDeployment();

  const runningDeployment = deploymentsData?.deployments.find(
    (deployment) => deployment.status === 'Running'
  );

  // Env vars are read when a deployment is created, so applying them means a
  // new deployment from the running one's image; a restart keeps the old values.
  const applyToRunningDeployment = async (deploymentId: string) => {
    try {
      await rollbackDeployment.mutateAsync({ appSlug, deploymentId });
      toast.success('Deploying the running image with the new environment');
    } catch (e: any) {
      toast.error(e?.message || 'Failed to apply environment variables');
    }
  };

  const extraGroups = useMemo<SuggestionGroup[]>(() => {
    const out: SuggestionGroup[] = [];
    for (const svc of suggestionsData?.services ?? []) {
      out.push({
        label: svc.name,
        items: svc.env_keys.map((k) => `${svc.name}.${k}`),
      });
    }
    out.push({ label: 'SLASHA', items: APP_SLASHA_REFS });
    return out;
  }, [suggestionsData]);

  const handleSave = async (vars: Record<string, string>) => {
    try {
      await updateEnvVars.mutateAsync({
        appSlug,
        vars,
      });
      if (!runningDeployment) {
        toast.success('Environment variables saved', {
          description: 'They take effect on the next deployment.',
        });
      } else {
        const deploymentId = runningDeployment.id;
        toast.success('Environment variables saved', {
          description:
            'The running deployment still has the old values. Apply them now, or they take effect on the next deployment.',
          duration: 15000,
          action: {
            label: 'Apply now',
            onClick: () => applyToRunningDeployment(deploymentId),
          },
        });
      }
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'env-vars'],
      });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save environment variables');
    }
  };

  return (
    <EnvEditor
      initialVars={envData?.env_vars ?? {}}
      isLoading={envLoading}
      isSaving={updateEnvVars.isPending}
      onSave={handleSave}
      extraGroups={extraGroups}
    />
  );
}
