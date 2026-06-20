import { useEffect, useRef, useState } from 'react';
import { Terminal, CircleDashed } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '~/components/interface/dialog';
import { HStack } from '~/components/interface/stacks';
import { getAuthToken } from '~/utils/jwt';

type LogStreamDialogProps = {
  url: string;
  title: string;
  onClose: () => void;
};

export function LogStreamDialog(props: LogStreamDialogProps) {
  const { url, title, onClose } = props;
  const [logs, setLogs] = useState<string[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const token = getAuthToken();
    const es = new EventSource(`${url}?token=${token}`);

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
  }, [url]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent className="flex h-[80vh] w-full max-w-4xl flex-col gap-0 border-border bg-bg p-0">
        <HStack
          justifyContent="between"
          className="shrink-0 border-b border-border p-4"
        >
          <HStack space={3}>
            <Terminal className="size-4 text-text-tertiary" />
            <DialogTitle className="text-sm">{title}</DialogTitle>
          </HStack>
        </HStack>

        <div
          ref={scrollRef}
          className="flex-1 overflow-auto bg-black/40 p-6 font-mono text-[13px] leading-relaxed selection:bg-white/10"
        >
          {logs.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-text-tertiary">
              <CircleDashed className="size-5 animate-spin" />
              <p>Establishing log stream...</p>
            </div>
          ) : (
            <div className="space-y-1">
              {logs.map((log, i) => (
                <div
                  key={i}
                  className="whitespace-pre-wrap break-all text-text-secondary"
                >
                  <span className="mr-3 select-none text-text-tertiary">
                    {i + 1}
                  </span>
                  {log}
                </div>
              ))}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
