import { useEffect, useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { FileText, FileWarning, Download } from 'lucide-react';
import { codeToHtml } from 'shiki';
import { getFileContentOptions } from '~/queries/files';
import { VStack, HStack } from '~/components/interface/stacks';
import { Skeleton } from '~/components/interface/skeleton';
import { Button } from '~/components/interface/button';
import { inferLang, formatFileSize } from '~/utils/format';

interface CodeViewerProps {
  slug: string;
  filePath: string;
}

export function CodeViewer({ slug, filePath }: CodeViewerProps) {
  const { data, isLoading, error } = useQuery(
    getFileContentOptions(slug, filePath)
  );
  const [highlightedHtml, setHighlightedHtml] = useState('');
  const [isHighlighting, setIsHighlighting] = useState(false);

  const fileName = filePath.split('/').pop() ?? filePath;

  const downloadUrl = useMemo(() => {
    if (!data?.content) return null;
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
    if (!downloadUrl) return;
    const link = document.createElement('a');
    link.href = downloadUrl;
    link.download = fileName;
    link.click();
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
      <div className="flex flex-col items-center justify-center gap-3 py-20 text-text-tertiary">
        <FileWarning className="size-6" />
        <p className="text-xs">Failed to load file</p>
      </div>
    );
  }

  if (!data) {
    return null;
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <HStack space={2}>
          <FileText className="size-3.5 text-text-tertiary" />
          <span className="text-[13px] font-medium text-text">{fileName}</span>
        </HStack>

        <HStack space={4}>
          <span className="text-[11px] text-text-tertiary">
            {formatFileSize(data.size)}
            {data.is_truncated && ' (truncated)'}
          </span>

          <Button
            variant="ghost"
            color="neutral"
            size="sm"
            icon={<Download className="size-4" />}
            onClick={handleDownload}
            isDisabled={!data.content}
          />
        </HStack>
      </div>

      <div className="flex-1 overflow-auto">
        {data.is_binary ? (
          <div className="flex flex-col items-center justify-center gap-3 py-20 text-text-tertiary">
            <FileWarning className="size-6" />
            <p className="text-xs">Binary file — cannot be displayed</p>
          </div>
        ) : isHighlighting || !highlightedHtml ? (
          <pre className="p-4 text-[13px] leading-relaxed text-text-secondary">
            <code>{data.content}</code>
          </pre>
        ) : (
          <div
            className="shiki-wrapper overflow-auto text-[13px] leading-relaxed [&_pre]:!bg-transparent [&_pre]:p-4 [&_code]:block [&_.line]:min-h-[1.4em]"
            dangerouslySetInnerHTML={{ __html: highlightedHtml }}
          />
        )}
      </div>
    </div>
  );
}
