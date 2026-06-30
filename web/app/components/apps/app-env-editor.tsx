import { useMemo } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import {
  getAppEnvSuggestionsOptions,
  getAppEnvVarsOptions,
  useUpdateAppEnvVars,
} from '~/queries/apps';
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
      toast.success('Environment variables saved');
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
