import { useState, useEffect, useMemo, useId } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Plus,
  Trash2,
  Save,
  KeyRound,
  Copy,
  Check,
  FileText,
  Table as TableIcon,
} from 'lucide-react';
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
  RichValueInput,
  SERVICE_SLASHA_REFS,
  type SuggestionGroup,
} from '~/components/apps/env-value-input';
import {
  type EnvVar,
  fromEnvRecord,
  toEnvRecord,
  parseDotEnv,
  serializeDotEnv,
  looksLikeDotEnv,
  noAutofillProps,
} from '~/components/apps/env-parsing';
import { EmptyState, RawEditor } from '~/components/apps/env-editor-parts';

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
  const [vars, setVars] = useState<EnvVar[]>(() => fromEnvRecord(initialVars));
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [mode, setMode] = useState<'table' | 'raw'>('table');
  const [rawText, setRawText] = useState<string>('');
  const [keyColWidth, setKeyColWidth] = useState<number>(240);
  const formId = useId();

  const startResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = keyColWidth;
    const onMove = (ev: MouseEvent) => {
      const next = Math.max(120, Math.min(600, startW + (ev.clientX - startX)));
      setKeyColWidth(next);
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  };

  const gridStyle = {
    gridTemplateColumns: `${keyColWidth}px 1fr auto`,
  };

  useEffect(() => {
    if (JSON.stringify(toEnvRecord(vars)) !== JSON.stringify(initialVars)) {
      setVars(fromEnvRecord(initialVars));
    }
  }, [initialVars]);

  const groupsForRow = (rowIndex: number): SuggestionGroup[] => {
    const ownKeys: string[] = [];
    for (let i = 0; i < vars.length; i++) {
      if (i === rowIndex) {
        continue;
      }
      const k = vars[i].key.trim();
      if (k && !ownKeys.includes(k)) {
        ownKeys.push(k);
      }
    }
    const groups: SuggestionGroup[] = [];
    if (ownKeys.length > 0) {
      groups.push({ label: 'Own', items: ownKeys });
    }
    if (extraGroups) {
      for (const g of extraGroups) {
        if (g.items.length > 0) {
          groups.push(g);
        }
      }
    }
    return groups;
  };

  const duplicateKeys = useMemo(() => {
    const seen = new Map<string, number>();
    const dupes = new Set<string>();
    for (const v of vars) {
      const k = v.key.trim();
      if (!k) {
        continue;
      }
      const count = (seen.get(k) ?? 0) + 1;
      seen.set(k, count);
      if (count > 1) {
        dupes.add(k);
      }
    }
    return dupes;
  }, [vars]);

  const commitVars = (newVars: EnvVar[]) => {
    setVars(newVars);
    if (onChange) {
      onChange(toEnvRecord(newVars));
    }
  };

  const enterRawMode = () => {
    setRawText(serializeDotEnv(vars));
    setMode('raw');
  };

  const exitRawMode = () => {
    setMode('table');
  };

  const handleRawChange = (text: string) => {
    setRawText(text);
    commitVars(parseDotEnv(text));
  };

  const handleSave = async () => {
    if (readOnly || !onSave) {
      return;
    }
    const finalVars = mode === 'raw' ? parseDotEnv(rawText) : vars;
    const keys = new Set<string>();
    for (const v of finalVars) {
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
    if (mode === 'raw') {
      setVars(finalVars);
      setMode('table');
    }
    await onSave(toEnvRecord(finalVars));
  };

  const handleCopy = (text: string, index: number) => {
    navigator.clipboard.writeText(text);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
    toast.success('Copied to clipboard');
  };

  const handleCopyAll = () => {
    navigator.clipboard.writeText(serializeDotEnv(vars));
    toast.success('Copied .env to clipboard');
  };

  const handleSmartPaste = (
    e: React.ClipboardEvent<HTMLInputElement | HTMLTextAreaElement>,
    rowIndex: number,
    field: 'key' | 'value'
  ) => {
    if (readOnly) {
      return;
    }
    const text = e.clipboardData.getData('text');
    if (!looksLikeDotEnv(text)) {
      return;
    }
    const parsed = parseDotEnv(text);
    if (parsed.length === 0) {
      return;
    }
    e.preventDefault();
    const current = vars[rowIndex];
    const isEmptyRow = current && !current.key.trim() && !current.value.trim();

    let newVars: EnvVar[];
    if (isEmptyRow) {
      newVars = [
        ...vars.slice(0, rowIndex),
        ...parsed,
        ...vars.slice(rowIndex + 1),
      ];
    } else if (
      field === 'value' &&
      parsed.length === 1 &&
      !text.includes('\n')
    ) {
      // Single KEY=VALUE pasted into a value cell — let it through as-is.
      return;
    } else {
      newVars = [...vars, ...parsed];
    }
    commitVars(newVars);
    toast.success(
      `Imported ${parsed.length} variable${parsed.length === 1 ? '' : 's'}`
    );
  };

  if (isLoading) {
    return (
      <div className="flex h-32 items-center justify-center text-text-tertiary">
        <span className="animate-pulse">Loading environment variables...</span>
      </div>
    );
  }

  const showHeader = variant === 'default';
  const showFooter = variant === 'default';

  const headerActions = (
    <HStack space={1.5}>
      {vars.length > 0 && (
        <Button
          label={mode === 'raw' ? 'Table view' : 'Edit as .env'}
          icon={
            mode === 'raw' ? (
              <TableIcon className="size-3.5" />
            ) : (
              <FileText className="size-3.5" />
            )
          }
          variant="ghost"
          size="sm"
          onClick={() => {
            if (mode === 'raw') {
              exitRawMode();
            } else {
              enterRawMode();
            }
          }}
        />
      )}
      {readOnly && vars.length > 0 && (
        <Button
          label="Copy .env"
          icon={<Copy className="size-3.5" />}
          variant="ghost"
          size="sm"
          onClick={handleCopyAll}
        />
      )}
    </HStack>
  );

  return (
    <VStack space={variant === 'embedded' ? 3 : 4}>
      <div
        className={cn(
          variant === 'default' &&
            'overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm'
        )}
      >
        {showHeader && (
          <div className="border-b border-border bg-surface/50 px-6 py-5">
            <HStack justifyContent="between" alignItems="start">
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
              {headerActions}
            </HStack>
          </div>
        )}

        <div className={cn(showHeader ? 'p-6' : 'p-0')}>
          {!showHeader && vars.length > 0 && !readOnly && (
            <div className="mb-3 flex justify-end">{headerActions}</div>
          )}

          {mode === 'raw' ? (
            <RawEditor
              value={rawText}
              onChange={handleRawChange}
              readOnly={readOnly}
            />
          ) : vars.length === 0 ? (
            <EmptyState
              readOnly={readOnly}
              onAdd={() => commitVars([{ key: '', value: '' }])}
              onPaste={(e) => {
                if (readOnly) {
                  return;
                }
                const text = e.clipboardData.getData('text');
                if (!looksLikeDotEnv(text)) {
                  return;
                }
                const parsed = parseDotEnv(text);
                if (parsed.length === 0) {
                  return;
                }
                e.preventDefault();
                commitVars(parsed);
                toast.success(
                  `Imported ${parsed.length} variable${
                    parsed.length === 1 ? '' : 's'
                  }`
                );
              }}
            />
          ) : (
            <VStack space={3}>
              <div className="overflow-hidden rounded-lg border border-border bg-surface/10">
                <div
                  className="relative grid gap-px border-b border-border bg-surface/50 text-[11px] font-medium uppercase tracking-wider text-text-tertiary"
                  style={gridStyle}
                >
                  <div className="border-r border-border px-4 py-2">Key</div>
                  <div className="px-4 py-2">Value</div>
                  <div className="w-10" />
                  {!readOnly && (
                    <div
                      onMouseDown={startResize}
                      className="absolute top-0 bottom-0 z-10 w-2 cursor-col-resize hover:bg-blue-500/20"
                      style={{ left: `${keyColWidth - 4}px` }}
                      aria-label="Resize key column"
                      role="separator"
                    />
                  )}
                </div>
                <div className="divide-y divide-border">
                  {vars.map((v, i) => {
                    const trimmedKey = v.key.trim();
                    const isDuplicate =
                      !!trimmedKey && duplicateKeys.has(trimmedKey);
                    return (
                      <div
                        key={i}
                        style={gridStyle}
                        className={cn(
                          'grid items-stretch gap-px transition-colors',
                          isDuplicate
                            ? 'bg-red-500/[0.04] hover:bg-red-500/[0.07]'
                            : 'hover:bg-white/[0.02]'
                        )}
                      >
                        <div
                          className={cn(
                            'border-r border-border px-2 py-2',
                            isDuplicate
                              ? 'bg-red-500/[0.03]'
                              : 'bg-white/[0.01]'
                          )}
                        >
                          <input
                            type="text"
                            placeholder="DATABASE_URL"
                            value={v.key}
                            name={`${formId}-key-${i}`}
                            readOnly={readOnly}
                            onChange={(e) => {
                              if (readOnly) {
                                return;
                              }
                              const newVars = [...vars];
                              newVars[i] = {
                                ...newVars[i],
                                key: e.target.value,
                              };
                              commitVars(newVars);
                            }}
                            onPaste={(e) => handleSmartPaste(e, i, 'key')}
                            {...noAutofillProps}
                            className={cn(
                              'w-full bg-transparent font-mono text-[13px] tracking-tight text-text outline-none placeholder:text-text-tertiary/60',
                              isDuplicate && 'text-red-300'
                            )}
                          />
                        </div>
                        <div className="min-w-0 px-2 py-1.5">
                          <HStack space={2} alignItems="start">
                            <div className="min-w-0 flex-1">
                              <RichValueInput
                                value={v.value}
                                placeholder="postgres://..."
                                readOnly={readOnly}
                                groups={groupsForRow(i)}
                                onChange={(val) => {
                                  if (readOnly) {
                                    return;
                                  }
                                  const newVars = [...vars];
                                  newVars[i] = {
                                    ...newVars[i],
                                    key: newVars[i].key,
                                    value: val,
                                  };
                                  commitVars(newVars);
                                }}
                                onPasteRaw={(text) => {
                                  if (readOnly) {
                                    return false;
                                  }
                                  if (!looksLikeDotEnv(text)) {
                                    return false;
                                  }
                                  const parsed = parseDotEnv(text);
                                  if (parsed.length === 0) {
                                    return false;
                                  }
                                  const current = vars[i];
                                  const isEmptyRow =
                                    current &&
                                    !current.key.trim() &&
                                    !current.value.trim();
                                  const newVars = isEmptyRow
                                    ? [
                                        ...vars.slice(0, i),
                                        ...parsed,
                                        ...vars.slice(i + 1),
                                      ]
                                    : [...vars, ...parsed];
                                  commitVars(newVars);
                                  toast.success(
                                    `Imported ${parsed.length} variable${
                                      parsed.length === 1 ? '' : 's'
                                    }`
                                  );
                                  return true;
                                }}
                              />
                            </div>
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
                                className="size-7 shrink-0"
                              />
                            )}
                          </HStack>
                        </div>
                        <div className="flex items-start justify-center px-1 py-2">
                          {!readOnly && (
                            <Button
                              variant="ghost"
                              color="error"
                              icon={<Trash2 className="size-3.5" />}
                              onClick={() => {
                                commitVars(
                                  vars.filter((_, index) => index !== i)
                                );
                              }}
                              className="size-7 opacity-50 hover:opacity-100"
                            />
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>

              {!readOnly && (
                <HStack justifyContent="between" alignItems="center">
                  <Button
                    label="Add Variable"
                    icon={<Plus className="size-3.5" />}
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      commitVars([...vars, { key: '', value: '' }])
                    }
                    className="h-8 text-[12px]"
                  />
                  <span className="text-[11px] text-text-tertiary/70">
                    Tip: paste a <code className="font-mono">.env</code> blob to
                    import multiple at once
                  </span>
                </HStack>
              )}
            </VStack>
          )}
        </div>

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
