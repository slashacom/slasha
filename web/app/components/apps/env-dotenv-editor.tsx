import { useEffect, useRef, useState } from 'react';
import { useEditor, EditorContent, ReactRenderer } from '@tiptap/react';
import Document from '@tiptap/extension-document';
import Paragraph from '@tiptap/extension-paragraph';
import Text from '@tiptap/extension-text';
import Placeholder from '@tiptap/extension-placeholder';
import Mention from '@tiptap/extension-mention';
import { cn } from '~/utils/classname';

const REF_RE = /\$\{\{\s*([\w.-]+)\s*\}\}/g;
const PARTIAL_RE = /\$\{\{\s*([\w.-]*)$/;

export type SuggestionGroup = {
  label: string;
  items: string[];
};

type FlatItem = {
  id: string;
  group: string;
};

function lineToContent(line: string) {
  const content: {
    type: string;
    text?: string;
    attrs?: { id: string; label: string };
  }[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  REF_RE.lastIndex = 0;
  while ((m = REF_RE.exec(line))) {
    if (m.index > last) {
      content.push({ type: 'text', text: line.slice(last, m.index) });
    }
    content.push({ type: 'mention', attrs: { id: m[1], label: m[1] } });
    last = REF_RE.lastIndex;
  }
  if (last < line.length) {
    content.push({ type: 'text', text: line.slice(last) });
  }
  return content.length === 0
    ? { type: 'paragraph' }
    : { type: 'paragraph', content };
}

function valueToContent(value: string) {
  const lines = value.split('\n');
  return {
    type: 'doc',
    content: lines.map((line) => lineToContent(line)),
  };
}

type HandlerBox = { current: (event: KeyboardEvent) => boolean };

type SuggestionListProps = {
  items: FlatItem[];
  command: (props: { id: string; label: string }) => void;
  handlerBox: HandlerBox;
};

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
    command({ id: item.id, label: item.id });
  };

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
        {items.map((item, i) => {
          const prevGroup = i > 0 ? items[i - 1].group : null;
          const showHeader = item.group !== prevGroup;
          return (
            <li key={`${item.group}::${item.id}`}>
              {showHeader && (
                <div className="px-3 pb-1 pt-2 text-[10px] font-medium uppercase tracking-wider text-text-tertiary/70">
                  {item.group}
                </div>
              )}
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
                <span>{item.id}</span>
                <span className="text-text-tertiary">&#125;&#125;</span>
              </button>
            </li>
          );
        })}
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

export type DotenvEditorProps = {
  value: string;
  onChange: (val: string) => void;
  groups: SuggestionGroup[];
  placeholder?: string;
  readOnly?: boolean;
};

export function DotenvEditor(props: DotenvEditorProps) {
  const { value, onChange, groups, placeholder, readOnly = false } = props;
  const groupsRef = useRef(groups);
  groupsRef.current = groups;
  const editor = useEditor(
    {
      extensions: [
        Document,
        Paragraph,
        Text,
        Placeholder.configure({ placeholder: placeholder ?? '' }),
        Mention.configure({
          HTMLAttributes: {
            class:
              'whitespace-nowrap rounded-[3px] bg-blue-500/10 px-0.5 font-mono text-[13px] text-blue-300',
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
                .run();
            },
            items: ({ query }) => {
              const q = query.toLowerCase();
              const out: FlatItem[] = [];
              for (const group of groupsRef.current) {
                for (const id of group.items) {
                  if (id.toLowerCase().includes(q)) {
                    out.push({ id, group: group.label });
                  }
                }
              }
              return out.slice(0, 12);
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
            'env-dotenv-editor block min-h-[120px] w-full bg-transparent font-mono text-[13px] leading-6 text-text outline-none [&_p]:m-0 [&_p]:min-h-[24px] [&_p]:break-words'
          ),
          autoComplete: 'off',
          autoCorrect: 'off',
          autoCapitalize: 'off',
          spellcheck: 'false',
          'data-1p-ignore': 'true',
          'data-lpignore': 'true',
          'data-form-type': 'other',
        },
      },
      onUpdate: ({ editor: ed }) => {
        onChange(ed.getText({ blockSeparator: '\n' }));
      },
    },
    [readOnly]
  );

  useEffect(() => {
    if (!editor) {
      return;
    }
    const current = editor.getText({ blockSeparator: '\n' });
    if (current !== value) {
      editor.commands.setContent(valueToContent(value), { emitUpdate: false });
    }
  }, [value, editor]);

  if (typeof document === 'undefined') {
    return (
      <pre className="min-h-[120px] w-full whitespace-pre-wrap font-mono text-[13px] leading-6 text-text">
        {value}
      </pre>
    );
  }

  return <EditorContent editor={editor} />;
}

export const APP_SLASHA_REFS = [
  'SLASHA.app_container_name',
  'SLASHA.app_id',
  'SLASHA.app_name',
  'SLASHA.app_slug',
  'SLASHA.network_name',
];

export const SERVICE_SLASHA_REFS = [
  'SLASHA.service_container_name',
  'SLASHA.service_id',
  'SLASHA.service_name',
  'SLASHA.app_id',
  'SLASHA.network_name',
];
