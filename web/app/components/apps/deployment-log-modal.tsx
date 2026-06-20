import { useEffect, useRef, useState } from 'react';
import { Terminal, CircleDashed } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { HStack } from '~/components/interface/stacks';
import { getAuthToken } from '~/utils/jwt';

type LogModalProps = {
  deploymentId: string;
  appSlug: string;
  onClose: () => void;
};

export function LogModal(props: LogModalProps) {
  const { deploymentId, appSlug, onClose } = props;
  const [logs, setLogs] = useState<string[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const token = getAuthToken();
    const url = `/api/apps/${appSlug}/deployments/${deploymentId}/logs?token=${token}`;
    const es = new EventSource(url);

    es.onmessage = (event) => {
      const data = event.data;
      if (data && data !== '[done]') {
        setLogs((prev) => [...prev, data]);
      }
    };

    es.onerror = (e) => {
      console.error('SSE Stream error:', e);
    };

    return () => {
      es.close();
    };
  }, [appSlug, deploymentId]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 px-4 py-8 backdrop-blur-sm">
      <div
        ref={containerRef}
        className="flex h-full w-full max-w-4xl flex-col rounded-lg border border-border bg-bg shadow-2xl"
      >
        <HStack
          justifyContent="between"
          className="shrink-0 border-b border-border p-4"
        >
          <HStack space={3}>
            <Terminal className="size-4 text-text-tertiary" />
            <h3 className="text-sm font-semibold text-text">Logs</h3>
          </HStack>
          <Button label="Close" variant="ghost" size="sm" onClick={onClose} />
        </HStack>

        <div
          ref={scrollRef}
          className="flex-1 overflow-auto bg-black/40 p-6 font-mono text-[13px] leading-relaxed selection:bg-white/10"
        >
          {logs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full gap-3 text-text-tertiary">
              <CircleDashed className="size-5 animate-spin" />
              <p>Establishing log stream...</p>
            </div>
          ) : (
            <div className="space-y-1">
              {logs.map((log, i) => (
                <div
                  key={i}
                  className="text-text-secondary whitespace-pre-wrap break-all"
                >
                  <span className="text-text-tertiary mr-3 select-none">
                    {i + 1}
                  </span>
                  {log}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
