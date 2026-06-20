import { useState, useEffect, useMemo } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Save, KeyRound } from 'lucide-react';
import { toast } from 'sonner';

import {
  getAppEnvSuggestionsOptions,
  getAppEnvVarsOptions,
  useUpdateAppEnvVars,
} from '~/queries/apps';
import {
  getServiceEnvVarsOptions,
  useUpdateServiceEnvVars,
} from '~/queries/services';

import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import {
  APP_SLASHA_REFS,
  DotenvEditor,
  SERVICE_SLASHA_REFS,
  type SuggestionGroup,
} from '~/components/apps/env-dotenv-editor';
import {
  fromEnvRecord,
  toEnvRecord,
  parseDotEnv,
  serializeDotEnv,
} from '~/components/apps/env-parsing';

export type EnvEditorProps = {
  title?: string;
  description?: string;
  initialVars: Record<string, string>;
  isLoading: boolean;
  isSaving: boolean;
  onSave?: (vars: Record<string, string>) => Promise<void> | void;
  onChange?: (vars: Record<string, string>) => void;
  onCancel?: () => void;
  readOnly?: boolean;
  variant?: 'default' | 'embedded';
  extraGroups?: SuggestionGroup[];
};

export function EnvEditor(props: EnvEditorProps) {
  const {
    title = 'Environment Variables',
    description = 'Define environment variables. These will be injected at runtime.',
    initialVars,
    isLoading,
    isSaving,
    onSave,
    onChange,
    onCancel,
    readOnly = false,
    variant = 'default',
    extraGroups,
  } = props;
  const [text, setText] = useState<string>(() =>
    serializeDotEnv(fromEnvRecord(initialVars))
  );

  useEffect(() => {
    if (
      JSON.stringify(toEnvRecord(parseDotEnv(text))) !==
      JSON.stringify(initialVars)
    ) {
      setText(serializeDotEnv(fromEnvRecord(initialVars)));
    }
  }, [initialVars]);

  const groups = useMemo<SuggestionGroup[]>(() => {
    const ownKeys: string[] = [];
    for (const v of parseDotEnv(text)) {
      const k = v.key.trim();
      if (k && !ownKeys.includes(k)) {
        ownKeys.push(k);
      }
    }
    const out: SuggestionGroup[] = [];
    if (ownKeys.length > 0) {
      out.push({ label: 'Own', items: ownKeys });
    }
    for (const g of extraGroups ?? []) {
      if (g.items.length > 0) {
        out.push(g);
      }
    }
    return out;
  }, [text, extraGroups]);

  const handleTextChange = (next: string) => {
    if (readOnly) {
      return;
    }
    setText(next);
    if (onChange) {
      onChange(toEnvRecord(parseDotEnv(next)));
    }
  };

  const handleSave = async () => {
    if (readOnly || !onSave) {
      return;
    }
    const parsed = parseDotEnv(text);
    const keys = new Set<string>();
    for (const v of parsed) {
      if (!v.key.trim()) {
        toast.error('Keys cannot be empty');
        return;
      }
      if (keys.has(v.key.trim())) {
        toast.error(`Duplicate key: ${v.key}`);
        return;
      }
      keys.add(v.key.trim());
    }
    await onSave(toEnvRecord(parsed));
  };

  const isEmbedded = variant === 'embedded';
  const showFooter = !isEmbedded && (!readOnly || !!onCancel);

  const editor = isLoading ? (
    <div className="flex h-24 items-center justify-center text-text-tertiary">
      <span className="animate-pulse">Loading environment variables...</span>
    </div>
  ) : (
    <div
      className={cn(
        'px-3.5 py-3',
        !isEmbedded &&
          'rounded-lg border border-border bg-bg/40 focus-within:border-text-tertiary/30'
      )}
    >
      <DotenvEditor
        value={text}
        onChange={handleTextChange}
        groups={groups}
        readOnly={readOnly}
        placeholder={
          'DATABASE_URL=postgres://...\nAPI_KEY=sk-...\n# Reference others with ${{ OTHER_VAR }}'
        }
      />
    </div>
  );

  return (
    <VStack space={isEmbedded ? 3 : 4}>
      <div
        className={cn(
          !isEmbedded &&
            'overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm'
        )}
      >
        {!isEmbedded && (
          <div className="border-b border-border bg-surface/50 px-6 py-5">
            <HStack space={3} alignItems="start">
              <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
                <KeyRound className="size-5" />
              </div>
              <div>
                <h3 className="text-[15px] font-semibold text-text">{title}</h3>
                <p className="mt-0.5 text-[13px] text-text-tertiary">
                  {description}
                </p>
              </div>
            </HStack>
          </div>
        )}

        <div className={cn(!isEmbedded && 'px-6 py-5')}>{editor}</div>

        {showFooter && (
          <div className="flex justify-end gap-3 border-t border-border bg-surface/50 px-6 py-4">
            {onCancel && (
              <Button
                label={readOnly ? 'Close' : 'Cancel'}
                variant="ghost"
                onClick={onCancel}
                size="sm"
              />
            )}
            {!readOnly && (
              <Button
                label="Save Changes"
                icon={<Save className="size-4" />}
                onClick={handleSave}
                isLoading={isSaving}
                size="sm"
              />
            )}
          </div>
        )}
      </div>
    </VStack>
  );
}

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
      toast.error(
        e.response?.data?.error || 'Failed to save environment variables'
      );
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
