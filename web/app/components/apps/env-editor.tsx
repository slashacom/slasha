import { useState, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Plus, Trash2, Save, KeyRound } from 'lucide-react';
import { toast } from 'sonner';

import { getAppEnvVarsOptions, useUpdateAppEnvVars } from '~/queries/apps';
import { getServiceEnvVarsOptions, useUpdateServiceEnvVars } from '~/queries/services';

import { Button } from '~/components/interface/button';
import { TextInput } from '~/components/interface/text-input';
import { HStack, VStack } from '~/components/interface/stacks';

export interface EnvVar {
  key: string;
  value: string;
}

export interface EnvEditorProps {
  title?: string;
  description?: string;
  initialVars: EnvVar[];
  isLoading: boolean;
  isSaving: boolean;
  onSave: (vars: EnvVar[]) => Promise<void>;
  onCancel?: () => void;
}

export function EnvEditor({
  title = 'Environment Variables',
  description = 'Define environment variables. These will be injected at runtime.',
  initialVars,
  isLoading,
  isSaving,
  onSave,
  onCancel,
}: EnvEditorProps) {
  const [vars, setVars] = useState<EnvVar[]>([]);

  useEffect(() => {
    setVars(initialVars);
  }, [initialVars]);

  const handleSave = async () => {
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

    await onSave(vars);
  };

  if (isLoading) {
    return (
      <div className="flex h-32 items-center justify-center text-text-tertiary">
        <span className="animate-pulse">Loading environment variables...</span>
      </div>
    );
  }

  return (
    <VStack space={4}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
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

        <div className="p-6">
          <VStack space={3}>
            {vars.length === 0 ? (
              <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-10">
                <p className="text-sm text-text-tertiary">
                  No environment variables defined.
                </p>
                <Button
                  label="Add First Variable"
                  icon={<Plus className="size-4" />}
                  variant="ghost"
                  size="sm"
                  className="mt-3"
                  onClick={() => setVars([{ key: '', value: '' }])}
                />
              </div>
            ) : (
              <div className="rounded-lg border border-border overflow-hidden">
                <div className="grid grid-cols-[1fr_1.5fr_auto] gap-4 border-b border-border bg-surface/30 px-4 py-2.5 text-[12px] font-medium text-text-tertiary uppercase tracking-wider">
                  <div>Key</div>
                  <div>Value</div>
                  <div className="w-8"></div>
                </div>
                <div className="divide-y divide-border bg-surface/10">
                  {vars.map((v, i) => (
                    <div
                      key={i}
                      className="grid grid-cols-[1fr_1.5fr_auto] items-center gap-4 p-2 transition-colors hover:bg-white/[0.02]"
                    >
                      <TextInput
                        placeholder="e.g. DATABASE_URL"
                        value={v.key}
                        onChange={(val) => {
                          const newVars = [...vars];
                          newVars[i].key = val;
                          setVars(newVars);
                        }}
                        className="bg-transparent border-transparent hover:border-border focus:bg-surface focus:border-border font-mono text-[13px]"
                      />
                      <TextInput
                        placeholder="e.g. postgres://..."
                        value={v.value}
                        onChange={(val) => {
                          const newVars = [...vars];
                          newVars[i].value = val;
                          setVars(newVars);
                        }}
                        className="bg-transparent border-transparent hover:border-border focus:bg-surface focus:border-border font-mono text-[13px]"
                      />
                      <Button
                        variant="ghost"
                        color="error"
                        icon={<Trash2 className="size-4" />}
                        onClick={() => {
                          setVars(vars.filter((_, index) => index !== i));
                        }}
                        className="opacity-50 hover:opacity-100"
                      />
                    </div>
                  ))}
                </div>
              </div>
            )}

            {vars.length > 0 && (
              <div className="mt-2">
                <Button
                  label="Add Variable"
                  icon={<Plus className="size-4" />}
                  variant="ghost"
                  size="sm"
                  onClick={() => setVars([...vars, { key: '', value: '' }])}
                />
              </div>
            )}
          </VStack>
        </div>

        <div className="border-t border-border bg-surface/50 px-6 py-4 flex justify-end gap-3">
          {onCancel && (
            <Button
              label="Cancel"
              variant="ghost"
              onClick={onCancel}
              size="sm"
            />
          )}
          <Button
            label="Save Changes"
            icon={<Save className="size-4" />}
            onClick={handleSave}
            isLoading={isSaving}
            size="sm"
          />
        </div>
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

  const handleSave = async (vars: EnvVar[]) => {
    try {
      await updateEnvVars.mutateAsync({
        appSlug,
        vars: vars.map((v) => ({
          key: v.key.trim(),
          value: v.value,
        })),
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
      initialVars={envData?.env_vars ?? []}
      isLoading={envLoading}
      isSaving={updateEnvVars.isPending}
      onSave={handleSave}
    />
  );
}

export function ServiceEnvEditor({ appSlug, serviceId, serviceName, onSaveSuccess, onCancel }: { appSlug: string, serviceId: string, serviceName: string, onSaveSuccess?: () => void, onCancel?: () => void }) {
  const queryClient = useQueryClient();
  const { data: envData, isLoading: envLoading } = useQuery(
    getServiceEnvVarsOptions(appSlug, serviceId)
  );
  const updateEnvVars = useUpdateServiceEnvVars();

  const handleSave = async (vars: EnvVar[]) => {
    try {
      await updateEnvVars.mutateAsync({
        appSlug,
        serviceId,
        vars: vars.map((v) => ({
          key: v.key.trim(),
          value: v.value,
        })),
      });
      toast.success('Service environment variables saved');
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services', serviceId, 'env-vars'],
      });
      onSaveSuccess?.();
    } catch (e: any) {
      toast.error(
        e.response?.data?.error || 'Failed to save service environment variables'
      );
    }
  };

  return (
    <EnvEditor
      title={`${serviceName} Environment Variables`}
      description="Define environment variables injected into this specific service."
      initialVars={envData?.env_vars ?? []}
      isLoading={envLoading}
      isSaving={updateEnvVars.isPending}
      onSave={handleSave}
      onCancel={onCancel}
    />
  );
}
