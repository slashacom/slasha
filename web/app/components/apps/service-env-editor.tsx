import { useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import {
  getServiceEnvVarsOptions,
  useUpdateServiceEnvVars,
} from '~/queries/services';
import {
  SERVICE_SLASHA_REFS,
  type SuggestionGroup,
} from '~/components/apps/env-dotenv-editor';
import { EnvEditor } from '~/components/apps/env-editor';

type ServiceEnvEditorProps = {
  appSlug: string;
  serviceId: string;
  serviceName: string;
  readOnly?: boolean;
  onSaveSuccess?: () => void;
  onCancel?: () => void;
};

export function ServiceEnvEditor(props: ServiceEnvEditorProps) {
  const {
    appSlug,
    serviceId,
    serviceName,
    readOnly = false,
    onSaveSuccess,
    onCancel,
  } = props;
  const queryClient = useQueryClient();
  const { data: envData, isLoading: envLoading } = useQuery(
    getServiceEnvVarsOptions(appSlug, serviceId)
  );
  const updateEnvVars = useUpdateServiceEnvVars();

  const handleSave = async (vars: Record<string, string>) => {
    if (readOnly) {
      return;
    }
    try {
      await updateEnvVars.mutateAsync({
        appSlug,
        serviceId,
        vars,
      });
      toast.success('Service environment variables saved');
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId, 'env-vars'],
      });
      onSaveSuccess?.();
    } catch (e: any) {
      toast.error(
        e.response?.data?.error ||
          'Failed to save service environment variables'
      );
    }
  };

  const extraGroups: SuggestionGroup[] = [
    { label: 'SLASHA', items: SERVICE_SLASHA_REFS },
  ];

  return (
    <EnvEditor
      title={`${serviceName} Configuration`}
      description={
        readOnly
          ? 'View environment variables for this service.'
          : 'Define environment variables injected into this specific service.'
      }
      initialVars={envData?.env_vars ?? {}}
      isLoading={envLoading}
      isSaving={updateEnvVars.isPending}
      onSave={readOnly ? undefined : handleSave}
      onCancel={onCancel}
      readOnly={readOnly}
      extraGroups={extraGroups}
    />
  );
}
