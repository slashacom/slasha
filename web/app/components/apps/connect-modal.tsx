import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Copy, Check } from 'lucide-react';
import type { Service } from '~/models/service';
import { getServiceEnvVarsOptions } from '~/queries/services';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '~/components/interface/dialog';

type ConnectModalProps = {
  appSlug: string;
  service: Service;
  onClose: () => void;
};

type CopyBlockProps = {
  text: string;
};

function CopyBlock(props: CopyBlockProps) {
  const { text } = props;
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      toast.error('Failed to copy: ' + e);
    }
  };

  return (
    <div className="relative rounded-lg border border-border bg-black/40 p-3 pr-10 font-mono text-[12px] text-text">
      <code className="select-all break-all">{text}</code>
      <button
        type="button"
        onClick={handleCopy}
        aria-label="Copy"
        className="absolute right-2 top-2 cursor-pointer rounded p-1 text-text-tertiary transition-colors hover:bg-white/5 hover:text-text"
      >
        {copied ? (
          <Check className="size-3.5 text-emerald-400" />
        ) : (
          <Copy className="size-3.5" />
        )}
      </button>
    </div>
  );
}

const TABS = [
  { id: 'app', label: 'From your app' },
  { id: 'machine', label: 'From your machine' },
] as const;

type TabId = (typeof TABS)[number]['id'];

export function ConnectModal(props: ConnectModalProps) {
  const { appSlug, service, onClose } = props;
  const [tab, setTab] = useState<TabId>('app');
  const { data: envData } = useQuery(
    getServiceEnvVarsOptions(appSlug, service.id)
  );

  const envKeys = Object.keys(envData?.env_vars ?? {}).sort();
  const primaryKey = envKeys.includes('DATABASE_URL')
    ? 'DATABASE_URL'
    : (envKeys[0] ?? 'DATABASE_URL');
  const otherKeys = envKeys.filter((key) => key !== primaryKey);

  const appExample = `${primaryKey}=\${{ ${service.name}.${primaryKey} }}`;
  const proxyCommand = `slasha proxy --app ${appSlug} ${service.name}`;

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Connect to {service.name}</DialogTitle>
        </DialogHeader>

        <HStack
          space={1}
          className="mt-4 w-fit rounded-lg border border-border bg-surface/50 p-1"
        >
          {TABS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              onClick={() => setTab(entry.id)}
              className={cn(
                'cursor-pointer rounded-md px-3 py-1 text-[12px] font-medium transition-colors',
                tab === entry.id
                  ? 'bg-white/10 text-text'
                  : 'text-text-tertiary hover:text-text'
              )}
            >
              {entry.label}
            </button>
          ))}
        </HStack>

        {tab === 'app' ? (
          <VStack space={3} className="mt-4">
            <p className="text-xs leading-5 text-text-tertiary">
              {service.name} is attached to your app&apos;s private network.
              Reference its variables from your app&apos;s Environment Variables
              in the Settings tab.
            </p>
            <CopyBlock text={appExample} />
            {otherKeys.length > 0 ? (
              <p className="text-[11px] leading-5 text-text-tertiary">
                Other variables:{' '}
                {otherKeys.map((key, index) => (
                  <span key={key}>
                    <span className="font-mono text-text-secondary">
                      {`\${{ ${service.name}.${key} }}`}
                    </span>
                    {index < otherKeys.length - 1 ? ', ' : ''}
                  </span>
                ))}
              </p>
            ) : null}
          </VStack>
        ) : (
          <VStack space={3} className="mt-4">
            <p className="text-xs leading-5 text-text-tertiary">
              Open a secure tunnel from your local machine to {service.kind}. It
              rides the existing HTTPS connection — no firewall changes or
              exposed ports.
            </p>
            <CopyBlock text={proxyCommand} />
            <p className="text-[11px] leading-5 text-text-tertiary">
              Prints the local port and a ready-to-paste connection string. Add{' '}
              <span className="font-mono text-text-secondary">--no-secret</span>{' '}
              to mask the password in shell output.
            </p>
          </VStack>
        )}

        <DialogFooter className="mt-6">
          <Button label="Close" variant="ghost" onClick={onClose} />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
