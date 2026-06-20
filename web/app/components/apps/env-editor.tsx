import { useState, useEffect, useMemo } from 'react';
import { Save, KeyRound } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
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
    <DotenvEditor
      value={text}
      onChange={handleTextChange}
      groups={groups}
      readOnly={readOnly}
      placeholder="DATABASE_URL=postgres://…   ( reference others with ${{ OTHER_VAR }} )"
    />
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
