import { useState, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Plus, Trash2, Save, KeyRound, Copy, Check } from 'lucide-react';
import { toast } from 'sonner';

import { getAppEnvVarsOptions, useUpdateAppEnvVars } from '~/queries/apps';
import {
  getServiceEnvVarsOptions,
  useUpdateServiceEnvVars,
} from '~/queries/services';

import { Button } from '~/components/interface/button';
import { TextInput } from '~/components/interface/text-input';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

export const fromEnvRecord = (
  record: Record<string, string> | undefined
): { key: string; value: string }[] => {
  return Object.entries(record ?? {}).map(([key, value]) => ({ key, value }));
};

export const toEnvRecord = (
  vars: { key: string; value: string }[]
): Record<string, string> => {
  const record: Record<string, string> = {};
  vars.forEach((v) => {
    if (v.key.trim()) {
      record[v.key.trim()] = v.value;
    }
  });
  return record;
};

export interface EnvEditorProps {
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
}

export function EnvEditor({
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
}: EnvEditorProps) {
  const [vars, setVars] = useState<{ key: string; value: string }[]>(() =>
    fromEnvRecord(initialVars)
  );
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  useEffect(() => {
    if (JSON.stringify(toEnvRecord(vars)) !== JSON.stringify(initialVars)) {
      setVars(fromEnvRecord(initialVars));
    }
  }, [initialVars]);

  const updateVars = (newVars: { key: string; value: string }[]) => {
    setVars(newVars);

    if (onChange) {
      onChange(toEnvRecord(newVars));
    }
  };

  const handleSave = async () => {
    if (readOnly || !onSave) return;
    const keys = new Set();
    for (const v of vars) {
      if (!v.key.trim()) {
        toast.error('Keys cannot be empty');
        return;
      }
      if (keys.has(v.key)) {
        toast.error(`Duplicate key: ${v.key}`);
        return;
      }
      keys.add(v.key);
    }

    await onSave(toEnvRecord(vars));
  };

  const handleCopy = (text: string, index: number) => {
    navigator.clipboard.writeText(text);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
    toast.success('Copied to clipboard');
  };

  if (isLoading) {
    return (
      <div className="flex h-32 items-center justify-center text-text-tertiary">
        <span className="animate-pulse">Loading environment variables...</span>
      </div>
    );
  }

  return (
    <VStack space={variant === 'embedded' ? 3 : 4}>
      <div
        className={cn(
          variant === 'default' &&
            'overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm'
        )}
      >
        {variant === 'default' && (
          <div className="border-b border-border bg-surface/50 px-6 py-5">
            <HStack justifyContent="between">
              <HStack space={3}>
                <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
                  <KeyRound className="size-5" />
                </div>
                <div>
                  <h3 className="text-[15px] font-semibold text-text">
                    {title}
                  </h3>
                  <p className="mt-0.5 text-[13px] text-text-tertiary">
                    {description}
                  </p>
                </div>
              </HStack>
            </HStack>
          </div>
        )}

        <div className={cn(variant === 'default' ? 'p-6' : 'p-0')}>
          <VStack space={3}>
            {vars.length === 0 ? (
              <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-10">
                <p className="text-sm text-text-tertiary">
                  No environment variables defined.
                </p>
                {!readOnly && (
                  <Button
                    label="Add First Variable"
                    icon={<Plus className="size-4" />}
                    variant="ghost"
                    size="sm"
                    className="mt-3"
                    onClick={() => updateVars([{ key: '', value: '' }])}
                  />
                )}
              </div>
            ) : (
              <div className="rounded-lg border border-border overflow-hidden bg-surface/10">
                <div className="grid grid-cols-[1.2fr_2fr_auto] gap-px border-b border-border bg-surface/50 text-[11px] font-medium text-text-tertiary uppercase tracking-wider">
                  <div className="px-4 py-2 border-r border-border">Key</div>
                  <div className="px-4 py-2">Value</div>
                  <div className="w-10"></div>
                </div>
                <div className="divide-y divide-border">
                  {vars.map((v, i) => (
                    <div
                      key={i}
                      className="grid grid-cols-[1.2fr_2fr_auto] items-stretch gap-px transition-colors hover:bg-white/[0.02]"
                    >
                      <div className="border-r border-border bg-white/[0.01] px-1.5 py-1">
                        <TextInput
                          placeholder="e.g. DATABASE_URL"
                          value={v.key}
                          onChange={(val) => {
                            if (readOnly) return;
                            const newVars = [...vars];
                            newVars[i].key = val;
                            updateVars(newVars);
                          }}
                          readOnly={readOnly}
                          className={cn(
                            'bg-transparent border-none font-mono text-[12px] h-8 focus:ring-0',
                            !readOnly && 'hover:bg-white/[0.02]'
                          )}
                        />
                      </div>
                      <div className="px-1.5 py-1">
                        <HStack space={2} className="h-full">
                          <TextInput
                            placeholder="e.g. postgres://..."
                            value={v.value}
                            onChange={(val) => {
                              if (readOnly) return;
                              const newVars = [...vars];
                              newVars[i].value = val;
                              updateVars(newVars);
                            }}
                            readOnly={readOnly}
                            className={cn(
                              'flex-1 bg-transparent border-none font-mono text-[12px] h-8 focus:ring-0',
                              !readOnly && 'hover:bg-white/[0.02]'
                            )}
                          />
                          {readOnly && (
                            <Button
                              variant="ghost"
                              size="sm"
                              icon={
                                copiedIndex === i ? (
                                  <Check className="size-3.5 text-emerald-400" />
                                ) : (
                                  <Copy className="size-3.5" />
                                )
                              }
                              onClick={() => handleCopy(v.value, i)}
                              className="size-7"
                            />
                          )}
                        </HStack>
                      </div>
                      <div className="flex items-center justify-center px-1">
                        {!readOnly && (
                          <Button
                            variant="ghost"
                            color="error"
                            icon={<Trash2 className="size-3.5" />}
                            onClick={() => {
                              updateVars(
                                vars.filter((_, index) => index !== i)
                              );
                            }}
                            className="size-7 opacity-50 hover:opacity-100"
                          />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {!readOnly && vars.length > 0 && (
              <div className="mt-1">
                <Button
                  label="Add Variable"
                  icon={<Plus className="size-3.5" />}
                  variant="ghost"
                  size="sm"
                  onClick={() => updateVars([...vars, { key: '', value: '' }])}
                  className="h-8 text-[12px]"
                />
              </div>
            )}
          </VStack>
        </div>

        {variant === 'default' && (
          <div className="border-t border-border bg-surface/50 px-6 py-4 flex justify-end gap-3">
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

export function AppEnvEditor({ appSlug }: { appSlug: string }) {
  const queryClient = useQueryClient();
  const { data: envData, isLoading: envLoading } = useQuery(
    getAppEnvVarsOptions(appSlug)
  );
  const updateEnvVars = useUpdateAppEnvVars();

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
    />
  );
}

export function ServiceEnvEditor({
  appSlug,
  serviceId,
  serviceName,
  readOnly = false,
  onSaveSuccess,
  onCancel,
}: {
  appSlug: string;
  serviceId: string;
  serviceName: string;
  readOnly?: boolean;
  onSaveSuccess?: () => void;
  onCancel?: () => void;
}) {
  const queryClient = useQueryClient();
  const { data: envData, isLoading: envLoading } = useQuery(
    getServiceEnvVarsOptions(appSlug, serviceId)
  );
  const updateEnvVars = useUpdateServiceEnvVars();

  const handleSave = async (vars: Record<string, string>) => {
    if (readOnly) return;
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
    />
  );
}
