import { useState } from 'react';
import { Copy, Check } from 'lucide-react';
import type { Service } from '~/models/service';
import { Button } from '~/components/interface/button';
import { VStack } from '~/components/interface/stacks';
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

