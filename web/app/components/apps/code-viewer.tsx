import { useEffect, useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { FileWarning, Download, Copy, Check } from 'lucide-react';
import { codeToHtml } from 'shiki';
import { getFileContentOptions } from '~/queries/files';
import { VStack, HStack } from '~/components/interface/stacks';
import { Skeleton } from '~/components/interface/skeleton';
import { Button } from '~/components/interface/button';
import { inferLang, formatFileSize } from '~/utils/format';
import { PathBreadcrumb } from './path-breadcrumb';

interface CodeViewerProps {
  slug: string;
  filePath: string;
  onNavigate: (path: string) => void;
}

export function CodeViewer(props: CodeViewerProps) {
  const { slug, filePath, onNavigate } = props;
  const { data, isLoading, error } = useQuery(
    getFileContentOptions(slug, filePath)
  );
  const [highlightedHtml, setHighlightedHtml] = useState('');
  const [isHighlighting, setIsHighlighting] = useState(false);
  const [copied, setCopied] = useState(false);

  const segments = filePath.split('/');
  const fileName = segments[segments.length - 1] ?? filePath;

  const downloadUrl = useMemo(() => {
    if (!data?.content) {
      return null;
    }
    const blob = new Blob([data.content], { type: 'text/plain' });
    return URL.createObjectURL(blob);
  }, [data?.content]);

  useEffect(() => {
    return () => {
      if (downloadUrl) {
        URL.revokeObjectURL(downloadUrl);
      }
    };
  }, [downloadUrl]);

  const handleDownload = () => {
    if (!downloadUrl) {
      return;
    }
    const link = document.createElement('a');
    link.href = downloadUrl;
    link.download = fileName;
    link.click();
  };

  const handleCopy = async () => {
    if (!data?.content) {
      return;
    }
    try {
      await navigator.clipboard.writeText(data.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {}
  };

  useEffect(() => {
    if (!data?.content) {
      setHighlightedHtml('');
      return;
    }

    let cancelled = false;
    setIsHighlighting(true);

    const lang = inferLang(data.name);
    codeToHtml(data.content, {
      lang,
      theme: 'github-dark-default',
    })
      .then((html) => {
        if (!cancelled) {
          setHighlightedHtml(html);
          setIsHighlighting(false);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setHighlightedHtml('');
          setIsHighlighting(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [data?.content, data?.name]);

  if (isLoading) {
    return (
      <VStack space={3} className="p-6">
        <Skeleton className="h-4 w-48 bg-white/5" />
        <Skeleton className="h-64 w-full bg-white/5" />
      </VStack>
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-text-tertiary">
        <FileWarning className="size-6" />
        <p className="text-xs">Failed to load file</p>
      </div>
    );
  }

  if (!data) {
    return null;
  }

  const lineCount = data.content ? data.content.split('\n').length : 0;

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
        <PathBreadcrumb
          path={filePath}
          isDirectory={false}
          onNavigate={onNavigate}
        />

        <HStack space={3} className="shrink-0">
          <span className="text-[11px] text-text-tertiary">
            {lineCount > 0 && `${lineCount} lines · `}
            {formatFileSize(data.size)}
            {data.is_truncated && ' (truncated)'}
          </span>

          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              color="neutral"
              size="sm"
              icon={
                copied ? (
                  <Check className="size-4 text-emerald-400" />
                ) : (
                  <Copy className="size-4" />
                )
              }
              onClick={handleCopy}
              isDisabled={!data.content}
            />
            <Button
              variant="ghost"
              color="neutral"
              size="sm"
              icon={<Download className="size-4" />}
              onClick={handleDownload}
              isDisabled={!data.content}
            />
          </div>
        </HStack>
      </div>

      <div className="flex-1 overflow-auto">
        {data.is_binary ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 text-text-tertiary">
            <FileWarning className="size-6" />
            <p className="text-xs">Binary file — cannot be displayed</p>
          </div>
        ) : isHighlighting || !highlightedHtml ? (
          <pre className="p-4 text-[13px] leading-relaxed text-text-secondary">
            <code>{data.content}</code>
          </pre>
        ) : (
          <div
            className="shiki-wrapper overflow-auto text-[13px] leading-[1.55]"
            dangerouslySetInnerHTML={{ __html: highlightedHtml }}
          />
        )}
      </div>
    </div>
  );
}
