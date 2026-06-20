import { ChevronRight, Folder } from 'lucide-react';
import { getFileIcon } from '~/utils/file-icon';

type PathBreadcrumbProps = {
  path: string;
  isDirectory: boolean;
  onNavigate: (path: string) => void;
}

export function PathBreadcrumb(props: PathBreadcrumbProps) {
  const { path, isDirectory, onNavigate } = props;
  const segments = path.split('/').filter(Boolean);
  const lastIdx = segments.length - 1;
  const lastName = segments[lastIdx] ?? '';
  const LeadingIcon = isDirectory ? Folder : getFileIcon(lastName);

  return (
    <div className="flex min-w-0 items-center gap-1 text-[12px]">
      {LeadingIcon ? (
        <LeadingIcon className="size-3.5 shrink-0 text-text-tertiary" />
      ) : null}
      {segments.map((segment, idx) => {
        const isLast = idx === lastIdx;
        const partial = segments.slice(0, idx + 1).join('/');
        return (
          <div key={idx} className="flex min-w-0 items-center gap-1">
            {isLast ? (
              <span className="truncate font-medium text-text">{segment}</span>
            ) : (
              <button
                type="button"
                onClick={() => {
                  onNavigate(partial);
                }}
                className="truncate text-text-tertiary transition-colors hover:text-text"
              >
                {segment}
              </button>
            )}
            {!isLast && (
              <ChevronRight className="size-3 shrink-0 text-text-tertiary/60" />
            )}
          </div>
        );
      })}
    </div>
  );
}
