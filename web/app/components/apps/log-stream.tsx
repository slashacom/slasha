import { useEffect, useRef, useState } from 'react';
import { CircleDashed } from 'lucide-react';
import { getAuthToken } from '~/utils/jwt';
import { cn } from '~/utils/classname';

type LogStreamProps = {
  url: string;
  className?: string;
};

export function LogStream(props: LogStreamProps) {
  const { url, className } = props;
  const [logs, setLogs] = useState<string[]>([]);
  const [done, setDone] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setLogs([]);
    setDone(false);
    const token = getAuthToken();
    const es = new EventSource(`${url}?token=${token}`);

    es.onmessage = (event) => {
      const data = event.data;
      // '[done]' marks the end of the historical replay; live lines keep
      // streaming after it, so we keep the connection open.
      if (data === '[done]') {
        setDone(true);
        return;
      }
      if (data) {
        setLogs((prev) => [...prev, data]);
      }
    };

    es.onerror = () => {
      es.close();
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
    <div
      ref={scrollRef}
      className={cn(
        'overflow-auto bg-black/40 p-6 font-mono text-[13px] leading-relaxed selection:bg-white/10',
        className
      )}
    >
      {logs.length === 0 ? (
        <div className="flex h-full flex-col items-center justify-center gap-3 text-text-tertiary">
          {done ? (
            <p>No logs for this deployment.</p>
          ) : (
            <>
              <CircleDashed className="size-5 animate-spin" />
              <p>Establishing log stream...</p>
            </>
          )}
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
  );
}
