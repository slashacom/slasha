import { useState } from 'react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { Textarea } from '~/components/interface/textarea';

interface NodeFormProps {
  initialData?: {
    name: string;
    host?: string | null;
    user?: string | null;
    port?: number | null;
  };
  onSubmit: (payload: {
    name: string;
    host?: string;
    user?: string;
    port?: number;
    ssh_private_key?: string;
  }) => void;
  onCancel: () => void;
  isPending: boolean;
  submitLabel: string;
  isLocalNode: boolean;
}

export function NodeForm({
  initialData,
  onSubmit,
  onCancel,
  isPending,
  submitLabel,
  isLocalNode,
}: NodeFormProps) {
  const [name, setName] = useState(initialData?.name ?? '');
  const [host, setHost] = useState(initialData?.host ?? '');
  const [user, setUser] = useState(initialData?.user ?? 'root');
  const [port, setPort] = useState(initialData?.port?.toString() ?? '22');
  const [sshPrivateKey, setSshPrivateKey] = useState('');

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const trimmedName = name.trim();
    if (!trimmedName) {
      toast.error('Node Name is required');
      return;
    }

    if (isLocalNode) {
      onSubmit({ name: trimmedName });
      return;
    }

    const trimmedHost = host.trim();
    const trimmedUser = user.trim();
    const parsedPort =
      port.trim() === '' ? undefined : Number.parseInt(port, 10);
    const trimmedKey = sshPrivateKey.trim();

    // For new node, sshPrivateKey is required. For edit, it's optional.
    const isNewNode = !initialData;
    if (isNewNode && (!trimmedHost || !trimmedUser || !trimmedKey)) {
      toast.error('All fields except Port are required');
      return;
    } else if (!isNewNode && (!trimmedHost || !trimmedUser)) {
      toast.error('Name, Host, and User are required');
      return;
    }

    if (
      parsedPort !== undefined &&
      (Number.isNaN(parsedPort) || parsedPort < 1 || parsedPort > 65535)
    ) {
      toast.error('Port must be a valid number between 1 and 65535');
      return;
    }

    const payload: {
      name: string;
      host?: string;
      user?: string;
      port?: number;
      ssh_private_key?: string;
    } = {
      name: trimmedName,
      host: trimmedHost,
      user: trimmedUser,
      port: parsedPort,
    };

    if (trimmedKey) {
      payload.ssh_private_key = trimmedKey;
    }

    onSubmit(payload);
  };

  const isFormValid = () => {
    if (!name.trim()) return false;
    if (isLocalNode) return true;

    if (!host.trim() || !user.trim()) return false;

    // For new node, ssh private key is required. For editing, it's optional.
    if (!initialData && !sshPrivateKey.trim()) return false;

    return true;
  };

  return (
    <form onSubmit={handleSubmit} className="w-full max-w-lg space-y-6">
      <div className="space-y-1.5">
        <Label
          htmlFor="name"
          className="text-[13px] font-medium text-text-secondary"
        >
          Node Name
        </Label>
        <Input
          id="name"
          type="text"
          required
          placeholder="worker-node-1"
          autoFocus
          className="h-10"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <p className="text-xs text-text-tertiary">
          A friendly nickname for this server.
        </p>
      </div>

      {!isLocalNode && (
        <>
          <div className="grid grid-cols-3 gap-4">
            <div className="col-span-2 space-y-1.5">
              <Label
                htmlFor="host"
                className="text-[13px] font-medium text-text-secondary"
              >
                Host Address / IP
              </Label>
              <Input
                id="host"
                type="text"
                required
                placeholder="192.168.1.100"
                className="h-10"
                value={host}
                onChange={(e) => setHost(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label
                htmlFor="port"
                className="text-[13px] font-medium text-text-secondary"
              >
                SSH Port
              </Label>
              <Input
                id="port"
                type="number"
                placeholder="22"
                className="h-10"
                value={port}
                onChange={(e) => setPort(e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label
              htmlFor="user"
              className="text-[13px] font-medium text-text-secondary"
            >
              SSH User
            </Label>
            <Input
              id="user"
              type="text"
              required
              placeholder="root"
              className="h-10"
              value={user}
              onChange={(e) => setUser(e.target.value)}
            />
            <p className="text-xs text-text-tertiary">
              The user account with passwordless sudo access.
            </p>
          </div>

          <div className="space-y-1.5">
            <Label
              htmlFor="ssh_private_key"
              className="text-[13px] font-medium text-text-secondary"
            >
              SSH Private Key
            </Label>
            <Textarea
              id="ssh_private_key"
              required={!initialData}
              placeholder={
                initialData
                  ? 'Leave empty to keep existing SSH key...'
                  : '-----BEGIN OPENSSH PRIVATE KEY-----\n...'
              }
              className="min-h-[150px] font-mono text-xs"
              value={sshPrivateKey}
              onChange={(e) => setSshPrivateKey(e.target.value)}
            />
            <p className="text-xs text-text-tertiary">
              {initialData
                ? 'Provide a new private key to replace the existing one, or leave blank to keep the current one.'
                : 'The private key used to authenticate. Slasha keeps this securely saved.'}
            </p>
          </div>
        </>
      )}

      <div className="flex items-center justify-end gap-2 pt-4">
        <Button
          variant="ghost"
          label="Cancel"
          type="button"
          onClick={onCancel}
          isDisabled={isPending}
        />
        <Button
          type="submit"
          label={submitLabel}
          isLoading={isPending}
          isDisabled={isPending || !isFormValid()}
        />
      </div>
    </form>
  );
}
