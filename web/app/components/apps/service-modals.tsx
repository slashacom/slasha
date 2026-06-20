import { useState, useEffect, useRef } from 'react';
import { Terminal, Copy, Check, CircleDashed } from 'lucide-react';
import type { Service } from '~/models/service';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { getAuthToken } from '~/utils/jwt';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '~/components/interface/dialog';
import { ServiceEnvEditor } from '~/components/apps/env-editor';

type ConnectModalProps = {
  appSlug: string;
  service: Service;
  onClose: () => void;
};

export function ConnectModal(props: ConnectModalProps) {
  const { appSlug, service, onClose } = props;
  const command = `slasha proxy --app ${appSlug} ${service.name}`;
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      toast.error('Failed to copy: ' + e);
    }
  };

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Connect to {service.name}</DialogTitle>
        </DialogHeader>
        <VStack space={4} className="mt-4">
          <p className="text-xs text-text-tertiary">
            Run this on your machine to open a secure tunnel to {service.kind}.
            The tunnel rides the existing HTTPS connection — no firewall rules
            required.
          </p>
          <div className="rounded-lg border border-border bg-black/40 p-3 font-mono text-[12px] text-text relative">
            <code className="select-all break-all pr-10">{command}</code>
            <button
              type="button"
              onClick={handleCopy}
              className="absolute top-2 right-2 rounded p-1 text-text-tertiary hover:text-text hover:bg-white/5 transition-colors"
              aria-label="Copy command"
            >
              {copied ? (
                <Check className="size-3.5 text-emerald-400" />
              ) : (
                <Copy className="size-3.5" />
              )}
            </button>
          </div>
          <p className="text-[11px] text-text-tertiary">
            The CLI prints the local port and a ready-to-paste connection
            string. Use <code>--no-secret</code> to mask the password in shell
            output.
          </p>
        </VStack>
        <DialogFooter className="mt-6">
          <Button label="Close" variant="ghost" onClick={onClose} />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

type ServiceConfigModalProps = {
  appSlug: string;
  service: Service;
  onClose: () => void;
};

export function ServiceConfigModal(props: ServiceConfigModalProps) {
  const { appSlug, service, onClose } = props;
  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl border-none bg-transparent p-0 shadow-none">
        <ServiceEnvEditor
          appSlug={appSlug}
          serviceId={service.id}
          serviceName={service.name}
          readOnly={true}
          onCancel={onClose}
        />
      </DialogContent>
    </Dialog>
  );
}

type ServiceLogModalProps = {
  serviceId: string;
  serviceName: string;
  appSlug: string;
  onClose: () => void;
};

export function ServiceLogModal(props: ServiceLogModalProps) {
  const { serviceId, serviceName, appSlug, onClose } = props;
  const [logs, setLogs] = useState<string[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const token = getAuthToken();
    const url = `/api/apps/${appSlug}/services/${serviceId}/logs?token=${token}`;
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
  }, [appSlug, serviceId]);

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
            <h3 className="text-sm font-semibold text-text">
              Service Logs — {serviceName}
            </h3>
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
