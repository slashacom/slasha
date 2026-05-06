import { useEffect, useState } from 'react';
import { useEditor, EditorContent, ReactRenderer } from '@tiptap/react';
import Document from '@tiptap/extension-document';
import Paragraph from '@tiptap/extension-paragraph';
import Text from '@tiptap/extension-text';
import Placeholder from '@tiptap/extension-placeholder';
import Mention from '@tiptap/extension-mention';
import { cn } from '~/utils/classname';

const SingleLineDocument = Document.extend({ content: 'paragraph' });
// Schema (single paragraph) already rejects splits, so we do not override
// Enter here — that lets the Mention suggestion plugin claim Enter when its
// popup is open.
const SingleLineParagraph = Paragraph;

const REF_RE = /\$\{\{\s*([\w.-]+)\s*\}\}/g;
const PARTIAL_RE = /\$\{\{\s*([\w.-]*)$/;

function valueToContent(value: string) {
  const content: { type: string; text?: string; attrs?: { id: string; label: string } }[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  REF_RE.lastIndex = 0;
  while ((m = REF_RE.exec(value))) {
    if (m.index > last) {
      content.push({ type: 'text', text: value.slice(last, m.index) });
    }
    content.push({ type: 'mention', attrs: { id: m[1], label: m[1] } });
    last = REF_RE.lastIndex;
  }
  if (last < value.length) {
    content.push({ type: 'text', text: value.slice(last) });
  }
  return {
    type: 'doc',
    content: [
      content.length === 0
        ? { type: 'paragraph' }
        : { type: 'paragraph', content },
    ],
  };
}

type HandlerBox = { current: (event: KeyboardEvent) => boolean };

interface SuggestionListProps {
  items: string[];
  command: (props: { id: string; label: string }) => void;
  handlerBox: HandlerBox;
}

function SuggestionList(props: SuggestionListProps) {
  const items = props.items;
  const command = props.command;
  const [selected, setSelected] = useState(0);

  useEffect(() => {
    setSelected(0);
  }, [items]);

  const select = (idx: number) => {
    const item = items[idx];
    if (!item) {
      return;
    }
    command({ id: item, label: item });
  };

  // Mutate the shared handler box so the suggestion plugin can route
  // keyboard events into the latest closure (selected, items, command).
  props.handlerBox.current = (event) => {
    if (items.length === 0) {
      return false;
    }
    if (event.key === 'ArrowDown') {
      setSelected((s) => (s + 1) % items.length);
      return true;
    }
    if (event.key === 'ArrowUp') {
      setSelected((s) => (s - 1 + items.length) % items.length);
      return true;
    }
    if (event.key === 'Enter' || event.key === 'Tab') {
      select(selected);
      return true;
    }
    return false;
  };

  if (items.length === 0) {
    return (
      <div className="rounded-md border border-border bg-bg/95 px-3 py-2 text-[12px] text-text-tertiary shadow-lg backdrop-blur">
        No matching variables
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-md border border-border bg-bg/95 shadow-lg backdrop-blur">
      <ul className="max-h-64 overflow-y-auto py-1">
        {items.map((item, i) => (
          <li key={item}>
            <button
              type="button"
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => select(i)}
              onMouseEnter={() => setSelected(i)}
              className={cn(
                'flex w-full items-center gap-2 px-3 py-1.5 text-left font-mono text-[12px] text-text',
                i === selected && 'bg-white/10'
              )}
            >
              <span className="text-text-tertiary">$&#123;&#123;</span>
              <span>{item}</span>
              <span className="text-text-tertiary">&#125;&#125;</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function positionPopup(el: HTMLDivElement, rect: DOMRect | null) {
  if (!rect) {
    return;
  }
  const margin = 6;
  const elHeight = el.offsetHeight;
  const elWidth = el.offsetWidth;
  let top = rect.bottom + margin;
  if (top + elHeight > window.innerHeight) {
    top = Math.max(margin, rect.top - elHeight - margin);
  }
  let left = rect.left;
  if (left + elWidth > window.innerWidth) {
    left = Math.max(margin, window.innerWidth - elWidth - margin);
  }
  el.style.top = `${top}px`;
  el.style.left = `${left}px`;
}

export interface RichValueInputProps {
  value: string;
  onChange: (val: string) => void;
  suggestions: string[];
  placeholder?: string;
  readOnly?: boolean;
  onPasteRaw?: (text: string) => boolean;
  className?: string;
}

export function RichValueInput({
  value,
  onChange,
  suggestions,
  placeholder,
  readOnly = false,
  onPasteRaw,
  className,
}: RichValueInputProps) {
  const editor = useEditor(
    {
      extensions: [
        SingleLineDocument,
        SingleLineParagraph,
        Text,
        Placeholder.configure({ placeholder: placeholder ?? '' }),
        Mention.configure({
          HTMLAttributes: {
            class:
              'inline-flex items-center rounded bg-blue-500/15 px-1 py-px font-mono text-[12px] text-blue-300 ring-1 ring-blue-500/25',
          },
          renderText({ node }) {
            return `\${{ ${node.attrs.id} }}`;
          },
          renderHTML({ options, node }) {
            return [
              'span',
              { ...options.HTMLAttributes, 'data-mention': node.attrs.id },
              `\${{ ${node.attrs.label} }}`,
            ];
          },
          suggestion: {
            allowSpaces: true,
            findSuggestionMatch: ({ $position }) => {
              const textBefore = $position.parent.textBetween(
                0,
                $position.parentOffset,
                undefined,
                '￼'
              );
              const m = textBefore.match(PARTIAL_RE);
              if (!m) {
                return null;
              }
              const from = $position.pos - m[0].length;
              return {
                range: { from, to: $position.pos },
                query: m[1] ?? '',
                text: m[0],
              };
            },
            command: ({ editor: ed, range, props }) => {
              const docSize = ed.state.doc.content.size;
              const lookahead = ed.state.doc.textBetween(
                range.to,
                Math.min(docSize, range.to + 4),
                '\n'
              );
              const trailingMatch = lookahead.match(/^\s*\}\}/);
              const trailing = trailingMatch ? trailingMatch[0].length : 0;
              ed.chain()
                .focus()
                .deleteRange({ from: range.from, to: range.to + trailing })
                .insertContent({
                  type: 'mention',
                  attrs: { id: props.id, label: props.label },
                })
                .insertContent(' ')
                .run();
            },
            items: ({ query }) => {
              const q = query.toLowerCase();
              return suggestions
                .filter((s) => s.toLowerCase().includes(q))
                .slice(0, 8);
            },
            render: () => {
              let renderer: ReactRenderer | null = null;
              let popup: HTMLDivElement | null = null;
              const handlerBox: HandlerBox = { current: () => false };
              return {
                onStart: (props) => {
                  renderer = new ReactRenderer(SuggestionList, {
                    props: { ...props, handlerBox },
                    editor: props.editor,
                  });
                  popup = document.createElement('div');
                  popup.style.position = 'fixed';
                  popup.style.zIndex = '60';
                  popup.appendChild(renderer.element);
                  document.body.appendChild(popup);
                  positionPopup(popup, props.clientRect?.() ?? null);
                },
                onUpdate: (props) => {
                  renderer?.updateProps({ ...props, handlerBox });
                  if (popup) {
                    positionPopup(popup, props.clientRect?.() ?? null);
                  }
                },
                onKeyDown: (props) => {
                  if (props.event.key === 'Escape') {
                    popup?.remove();
                    return true;
                  }
                  return handlerBox.current(props.event);
                },
                onExit: () => {
                  popup?.remove();
                  popup = null;
                  renderer?.destroy();
                  renderer = null;
                },
              };
            },
          },
        }),
      ],
      content: valueToContent(value),
      editable: !readOnly,
      editorProps: {
        attributes: {
          class: cn(
            'env-value-editor block w-full bg-transparent font-mono text-[13px] leading-5 text-text outline-none [&_p]:m-0 [&_p]:min-h-[20px]',
            className
          ),
          autoComplete: 'off',
          autoCorrect: 'off',
          autoCapitalize: 'off',
          spellcheck: 'false',
          'data-1p-ignore': 'true',
          'data-lpignore': 'true',
          'data-form-type': 'other',
        },
        handlePaste: (_view, event) => {
          if (!onPasteRaw) {
            return false;
          }
          const text = event.clipboardData?.getData('text');
          if (!text) {
            return false;
          }
          return onPasteRaw(text);
        },
      },
      onUpdate: ({ editor: ed }) => {
        onChange(ed.getText({ blockSeparator: '' }));
      },
    },
    [readOnly]
  );

  useEffect(() => {
    if (!editor) {
      return;
    }
    const current = editor.getText({ blockSeparator: '' });
    if (current !== value) {
      editor.commands.setContent(valueToContent(value), { emitUpdate: false });
    }
  }, [value, editor]);

  if (typeof document === 'undefined') {
    return (
      <div className={cn('font-mono text-[13px] text-text', className)}>
        {value}
      </div>
    );
  }

  return <EditorContent editor={editor} />;
}

export const SLASHA_SYSTEM_REFS = [
  'SLASHA.app_container_name',
  'SLASHA.app_id',
  'SLASHA.app_name',
  'SLASHA.app_slug',
  'SLASHA.network_name',
  'SLASHA.service_container_name',
];
