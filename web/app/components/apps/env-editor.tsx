import { useState, useEffect, useMemo, type ReactNode } from 'react';
import { Save, KeyRound } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '~/components/interface/button';
import { HStack } from '~/components/interface/stacks';
import {
  DotenvEditor,
  type SuggestionGroup,
} from '~/components/apps/env-dotenv-editor';
import {
  fromEnvRecord,
  toEnvRecord,
  parseDotEnv,
  serializeDotEnv,
} from '~/components/apps/env-parsing';
import { SettingsCard } from '~/components/interface/settings-card';

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
  hint?: ReactNode;
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
    hint,
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
    <DotenvEditor
      value={text}
      onChange={handleTextChange}
      groups={groups}
      readOnly={readOnly}
      padded={!isEmbedded}
      placeholder="DATABASE_URL=postgres://…   ( reference others with ${{ OTHER_VAR }} )"
    />
  );

  const body = (
    <>
      <div className="min-w-0">{editor}</div>

      {showFooter && (
        <div className="flex items-center justify-between gap-3 border-t border-border bg-surface/50 px-6 py-4">
          <div className="min-w-0 text-[12px] leading-5 text-text-tertiary">
            {hint}
          </div>
          <HStack space={3}>
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
          </HStack>
        </div>
      )}
    </>
  );

  if (isEmbedded) {
    return <div className="min-w-0">{body}</div>;
  }

  return (
    <SettingsCard
      icon={KeyRound}
      title={title}
      description={description}
      body={body}
    />
  );
}
