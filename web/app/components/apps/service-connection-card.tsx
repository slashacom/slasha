import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Eye, EyeOff, Pencil } from 'lucide-react';
import type { Service } from '~/models/service';
import { getServiceEnvVarsOptions } from '~/queries/services';
import { Button } from '~/components/interface/button';
import { CopyButton } from '~/components/interface/copy-button';
import { HStack, VStack } from '~/components/interface/stacks';
import { ServiceEnvEditor } from '~/components/apps/service-env-editor';
import {
  isServiceSecret,
  serviceEnvReference,
  serviceProxyCommand,
} from '~/utils/service-env';

const MASK = '••••••••••••';

type SnippetProps = {
  label: string;
  hint: string;
  value: string;
};

function Snippet(props: SnippetProps) {
  const { label, hint, value } = props;

  return (
    <VStack space={1.5} className="min-w-0">
      <span className="text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
        {label}
      </span>
      <HStack className="min-w-0 rounded-lg border border-border bg-black/40 pr-1.5">
        <code className="min-w-0 flex-1 overflow-x-auto whitespace-nowrap px-3 py-2 font-mono text-[12px] text-text custom-scrollbar">
          {value}
        </code>
        <CopyButton value={value} label={`Copy ${label}`} />
      </HStack>
      <span className="text-[11px] leading-5 text-text-tertiary">{hint}</span>
    </VStack>
  );
}

type VariableRowProps = {
  service: Service;
  envKey: string;
  value: string;
};

function VariableRow(props: VariableRowProps) {
  const { service, envKey, value } = props;
  const [revealed, setRevealed] = useState(false);
  const isSecret = isServiceSecret(service.kind, envKey);

  return (
    <HStack space={3} className="min-w-0 border-t border-border px-4 py-2">
      <code className="w-56 shrink-0 truncate font-mono text-[12px] text-text-secondary">
        {envKey}
      </code>
      <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-text">
        {isSecret && !revealed ? MASK : value}
      </code>
      {isSecret ? (
        <button
          type="button"
          onClick={() => setRevealed((current) => !current)}
          aria-label={revealed ? `Hide ${envKey}` : `Reveal ${envKey}`}
          className="flex size-6 shrink-0 cursor-pointer items-center justify-center rounded text-text-tertiary transition-colors hover:bg-white/5 hover:text-text"
        >
          {revealed ? (
            <EyeOff className="size-3" />
          ) : (
            <Eye className="size-3" />
          )}
        </button>
      ) : null}
      <CopyButton value={value} label={`Copy ${envKey}`} />
    </HStack>
  );
}

type ServiceConnectionCardProps = {
  appSlug: string;
  service: Service;
};

export function ServiceConnectionCard(props: ServiceConnectionCardProps) {
  const { appSlug, service } = props;
  const [isEditing, setEditing] = useState(false);
  const { data: envData, isLoading } = useQuery(
    getServiceEnvVarsOptions(appSlug, service.id)
  );

  if (isEditing) {
    return (
      <ServiceEnvEditor
        appSlug={appSlug}
        serviceId={service.id}
        serviceName={service.name}
        onSaveSuccess={() => setEditing(false)}
        onCancel={() => setEditing(false)}
      />
    );
  }

  const envVars = envData?.env_vars ?? {};
  const envKeys = Object.keys(envVars).sort();
  const reference = serviceEnvReference(service.name, 'DATABASE_URL');

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-surface/50">
      <HStack
        justifyContent="between"
        className="border-b border-border bg-white/[0.02] px-4 py-2"
      >
        <span className="text-xs font-semibold text-text">Connection</span>
        <Button
          label="Edit variables"
          variant="ghost"
          size="sm"
          icon={<Pencil className="size-3.5" />}
          onClick={() => setEditing(true)}
        />
      </HStack>

      <VStack space={4} className="p-4">
        <Snippet
          label="From your app"
          hint={`Slasha builds this from the variables below and injects it into containers on the app's private network. Individual variables are available as ${serviceEnvReference(service.name, 'KEY')}.`}
          value={reference}
        />
        {service.status === 'Running' ? (
          <Snippet
            label="From your machine"
            hint="Opens a tunnel over the existing HTTPS connection and prints a ready-to-paste connection string. No firewall changes or exposed ports."
            value={serviceProxyCommand(appSlug, service.name)}
          />
        ) : null}
      </VStack>

      <div className="border-t border-border">
        <HStack justifyContent="between" className="bg-white/[0.02] px-4 py-2">
          <span className="text-xs font-semibold text-text">Variables</span>
          <span className="text-[11px] text-text-tertiary">
            {envKeys.length} configured
          </span>
        </HStack>
        {isLoading ? (
          <div className="px-4 py-4 text-[12px] text-text-tertiary">
            Loading variables…
          </div>
        ) : envKeys.length === 0 ? (
          <div className="px-4 py-4 text-[12px] text-text-tertiary">
            No variables configured for this service.
          </div>
        ) : (
          envKeys.map((envKey) => (
            <VariableRow
              key={envKey}
              service={service}
              envKey={envKey}
              value={envVars[envKey]}
            />
          ))
        )}
      </div>
    </div>
  );
}
