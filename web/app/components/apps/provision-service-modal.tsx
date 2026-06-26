import { useState, useMemo, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { ChevronDown, ChevronRight, KeyRound, Loader2 } from 'lucide-react';
import type { ServiceKind } from '~/models/service';
import {
  getServiceKindsOptions,
  useProvisionService,
} from '~/queries/services';
import { Button } from '~/components/interface/button';
import { Select } from '~/components/interface/select';
import { HStack, VStack } from '~/components/interface/stacks';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '~/components/interface/dialog';
import { TextInput } from '~/components/interface/text-input';
import { EnvEditor } from '~/components/apps/env-editor';
import { EnvVarChips } from '~/components/apps/env-var-chips';
import { buildResourcesPayload } from '~/components/apps/service-resources';
import { primaryEnvKey, serviceEnvReference } from '~/utils/service-env';

type ProvisionServiceModalProps = {
  appSlug: string;
  onClose: () => void;
};

// Service names double as the reference namespace (`${{ name.KEY }}`), so keep
// them to a safe identifier: spaces become hyphens and other characters are
// dropped.
function sanitizeServiceName(value: string) {
  return value.replace(/\s+/g, '-').replace(/[^a-zA-Z0-9_-]/g, '');
}

export function ProvisionServiceModal(props: ProvisionServiceModalProps) {
  const { appSlug, onClose } = props;
  const { data } = useQuery(getServiceKindsOptions());
  const provisionService = useProvisionService();

  const kinds = data?.kinds ?? [];

  const [name, setName] = useState('');
  const [kindName, setKindName] = useState<ServiceKind | ''>('');
  const [version, setVersion] = useState<string>('');
  const [envVars, setEnvVars] = useState<Record<string, string>>({});

  const [isEnvOpen, setIsEnvOpen] = useState(false);
  const [isAdvancedOpen, setIsAdvancedOpen] = useState(false);
  const [memoryMb, setMemoryMb] = useState('');
  const [cpuCores, setCpuCores] = useState('');
  const [shmMb, setShmMb] = useState('');
  const [pidsLimit, setPidsLimit] = useState('');

  const selectedKind = useMemo(() => {
    return kinds.find((k) => k.name === kindName);
  }, [kinds, kindName]);

  const handleKindChange = (newKind: ServiceKind) => {
    setKindName(newKind);
    const kindMeta = kinds.find((k) => k.name === newKind);
    if (kindMeta) {
      setVersion(kindMeta.supported_versions[0]);
      setEnvVars(kindMeta.default_env_vars);
    }
  };

  useEffect(() => {
    if (kinds.length > 0 && !kindName) {
      handleKindChange(kinds[0].name);
    }
  }, [kinds, kindName]);

  const handleProvision = async () => {
    if (!name.trim() || !kindName || !version) {
      toast.error('Please fill in all fields.');
      return;
    }

    const { payload, error } = buildResourcesPayload(
      memoryMb,
      cpuCores,
      shmMb,
      pidsLimit
    );
    if (error) {
      toast.error(error);
      return;
    }

    try {
      await provisionService.mutateAsync({
        appSlug,
        kind: kindName,
        name: name.trim(),
        version,
        envVars,
        resources: payload,
      });
      onClose();
    } catch (e) {
      toast.error('Failed to provision service: ' + e);
    }
  };

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Provision Service</DialogTitle>
        </DialogHeader>
        <VStack space={4} className="mt-4 min-w-0">
          <VStack space={1.5}>
            <label className="text-xs font-medium text-text-secondary">
              Service Name
            </label>
            <TextInput
              value={name}
              onChange={(value) => setName(sanitizeServiceName(value))}
              placeholder="e.g. main-db"
            />
          </VStack>

          <HStack space={4}>
            <VStack space={1.5} className="flex-1">
              <label className="text-xs font-medium text-text-secondary">
                Type
              </label>
              <Select
                value={kindName}
                onChange={(e) =>
                  handleKindChange(e.target.value as ServiceKind)
                }
              >
                {kinds.map((k) => (
                  <option key={k.name} value={k.name}>
                    {k.name}
                  </option>
                ))}
              </Select>
            </VStack>

            <VStack space={1.5} className="w-1/3">
              <label className="text-xs font-medium text-text-secondary">
                Version
              </label>
              <Select
                value={version}
                onChange={(e) => setVersion(e.target.value)}
              >
                {selectedKind?.supported_versions.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </Select>
            </VStack>
          </HStack>

          <EnvConfigSection
            kindName={kindName}
            name={name}
            envVars={envVars}
            loading={!selectedKind}
            isOpen={isEnvOpen}
            onToggle={() => setIsEnvOpen((v) => !v)}
            onChange={setEnvVars}
          />

          <AdvancedResourcesSection
            isOpen={isAdvancedOpen}
            onToggle={() => setIsAdvancedOpen((v) => !v)}
            memoryMb={memoryMb}
            cpuCores={cpuCores}
            shmMb={shmMb}
            pidsLimit={pidsLimit}
            onMemoryChange={setMemoryMb}
            onCpuChange={setCpuCores}
            onShmChange={setShmMb}
            onPidsChange={setPidsLimit}
          />
        </VStack>
        <DialogFooter className="mt-6">
          <Button label="Cancel" variant="ghost" onClick={onClose} />
          <Button
            label="Provision"
            onClick={handleProvision}
            isLoading={provisionService.isPending}
            disabled={!name.trim() || !kindName || !version}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

type EnvConfigSectionProps = {
  kindName: ServiceKind | '';
  name: string;
  envVars: Record<string, string>;
  loading: boolean;
  isOpen: boolean;
  onToggle: () => void;
  onChange: (vars: Record<string, string>) => void;
};

function EnvConfigSection(props: EnvConfigSectionProps) {
  const { kindName, name, envVars, loading, isOpen, onToggle, onChange } =
    props;
  const keys = useMemo(() => Object.keys(envVars).sort(), [envVars]);
  const refName = name.trim() || '<service-name>';
  const hasSecret = keys.some((k) => /password|secret|token|key/i.test(k));

  return (
    <VStack space={2} className="mt-4 min-w-0">
      <HStack space={2} justifyContent="between">
        <label className="text-xs font-medium text-text-secondary">
          Environment Configuration
        </label>
        {!loading ? (
          <button
            type="button"
            onClick={onToggle}
            className="flex items-center gap-1 text-xs font-medium text-text-secondary transition-colors hover:text-text"
          >
            {isOpen ? (
              <ChevronDown className="size-3.5" />
            ) : (
              <ChevronRight className="size-3.5" />
            )}
            {isOpen ? 'Hide' : 'Customize'}
          </button>
        ) : null}
      </HStack>

      {loading ? (
        <div className="flex min-h-[135px] items-center justify-center rounded-lg border border-border bg-surface/30">
          <Loader2 className="size-4 animate-spin text-text-tertiary" />
        </div>
      ) : isOpen ? (
        <div className="max-h-[400px] overflow-auto rounded-lg border border-border bg-surface/30 custom-scrollbar">
          <EnvEditor
            key={kindName}
            initialVars={envVars}
            isLoading={false}
            isSaving={false}
            onChange={onChange}
            variant="embedded"
          />
        </div>
      ) : (
        <div className="min-h-[135px] rounded-lg border border-border bg-surface/30 p-4">
          <HStack space={3} alignItems="start">
            <div className="rounded-md bg-white/5 p-1.5 text-text-secondary">
              <KeyRound className="size-4" />
            </div>
            <VStack space={2} className="min-w-0 flex-1">
              <span className="text-[13px] text-text">
                {keys.length} variables will be configured for you
              </span>
              <EnvVarChips keys={keys} />
              {hasSecret ? (
                <span className="text-[11px] leading-5 text-text-tertiary">
                  Secrets like passwords are generated automatically.
                </span>
              ) : null}
            </VStack>
          </HStack>
        </div>
      )}

      <p className="text-[11px] leading-5 text-text-tertiary">
        Reference these from your app as{' '}
        <span className="font-mono text-text-secondary">
          {serviceEnvReference(refName, primaryEnvKey(keys))}
        </span>
        . Edit anytime from the service&apos;s settings.
      </p>
    </VStack>
  );
}

type AdvancedResourcesSectionProps = {
  isOpen: boolean;
  onToggle: () => void;
  memoryMb: string;
  cpuCores: string;
  shmMb: string;
  pidsLimit: string;
  onMemoryChange: (v: string) => void;
  onCpuChange: (v: string) => void;
  onShmChange: (v: string) => void;
  onPidsChange: (v: string) => void;
};

function AdvancedResourcesSection(props: AdvancedResourcesSectionProps) {
  const {
    isOpen,
    onToggle,
    memoryMb,
    cpuCores,
    shmMb,
    pidsLimit,
    onMemoryChange,
    onCpuChange,
    onShmChange,
    onPidsChange,
  } = props;
  return (
    <VStack space={2} className="mt-4">
      <button
        type="button"
        onClick={onToggle}
        className="flex items-center gap-1.5 text-xs font-medium text-text-secondary hover:text-text transition-colors w-fit"
      >
        {isOpen ? (
          <ChevronDown className="size-3.5" />
        ) : (
          <ChevronRight className="size-3.5" />
        )}
        Advanced
      </button>

      {isOpen && (
        <VStack
          space={3}
          className="rounded-lg border border-border bg-surface/30 p-4"
        >
          <p className="text-[11px] text-text-tertiary">
            Per-container resource caps. Leave blank for unlimited.
          </p>

          <HStack space={3}>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                Memory (MB)
              </label>
              <TextInput
                value={memoryMb}
                onChange={onMemoryChange}
                placeholder="unlimited"
              />
            </VStack>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                CPU (cores)
              </label>
              <TextInput
                value={cpuCores}
                onChange={onCpuChange}
                placeholder="unlimited"
              />
            </VStack>
          </HStack>

          <HStack space={3}>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                Shared Memory (MB)
              </label>
              <TextInput
                value={shmMb}
                onChange={onShmChange}
                placeholder="unlimited"
              />
            </VStack>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                PID Limit
              </label>
              <TextInput
                value={pidsLimit}
                onChange={onPidsChange}
                placeholder="unlimited"
              />
            </VStack>
          </HStack>
        </VStack>
      )}
    </VStack>
  );
}
