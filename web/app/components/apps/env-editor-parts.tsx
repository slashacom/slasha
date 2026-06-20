import { useRef } from 'react';
import { Plus } from 'lucide-react';
import TextareaAutosize from 'react-textarea-autosize';
import { Button } from '~/components/interface/button';
import { noAutofillProps } from '~/components/apps/env-parsing';

type EmptyStateProps = {
  readOnly: boolean;
  onAdd: () => void;
  onPaste: (e: React.ClipboardEvent<HTMLDivElement>) => void;
};

export function EmptyState(props: EmptyStateProps) {
  const { readOnly, onAdd, onPaste } = props;
  return (
    <div
      onPaste={onPaste}
      className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-10"
    >
      <p className="text-sm text-text-tertiary">
        No environment variables defined.
      </p>
      {!readOnly && (
        <>
          <p className="mt-1 text-[12px] text-text-tertiary/70">
            Add one manually, or paste a <code className="font-mono">.env</code>{' '}
            file here.
          </p>
          <Button
            label="Add First Variable"
            icon={<Plus className="size-4" />}
            variant="ghost"
            size="sm"
            className="mt-3"
            onClick={onAdd}
          />
        </>
      )}
    </div>
  );
}

type RawEditorProps = {
  value: string;
  onChange: (v: string) => void;
  readOnly: boolean;
};

export function RawEditor(props: RawEditorProps) {
  const { value, onChange, readOnly } = props;
  const ref = useRef<HTMLTextAreaElement>(null);
  return (
    <div className="overflow-hidden rounded-lg border border-border bg-surface/10">
      <div className="border-b border-border bg-surface/50 px-4 py-2 text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
        .env
      </div>
      <TextareaAutosize
        ref={ref}
        value={value}
        readOnly={readOnly}
        onChange={(e) => onChange(e.target.value)}
        minRows={8}
        maxRows={24}
        wrap="off"
        placeholder={
          'DATABASE_URL=postgres://...\nAPI_KEY=sk-...\n# comments are supported'
        }
        {...noAutofillProps}
        className="block w-full resize-none overflow-x-auto whitespace-pre bg-transparent px-4 py-3 font-mono text-[13px] leading-5 text-text outline-none placeholder:text-text-tertiary/50"
      />
    </div>
  );
}
