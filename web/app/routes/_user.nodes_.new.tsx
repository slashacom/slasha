import { useState } from 'react';
import { useNavigate, redirect } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { Textarea } from '~/components/interface/textarea';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';
import { useCreateNode } from '~/queries/nodes';

export async function clientLoader() {
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    throw redirect('/apps');
  }
  return null;
}

export default function NewNodePage() {
  const navigate = useNavigate();
  const createNode = useCreateNode();

  const [name, setName] = useState('');
  const [host, setHost] = useState('');
  const [user, setUser] = useState('root');
  const [port, setPort] = useState('22');
  const [sshPrivateKey, setSshPrivateKey] = useState('');

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const trimmedName = name.trim();
    const trimmedHost = host.trim();
    const trimmedUser = user.trim();
    const parsedPort =
      port.trim() === '' ? undefined : Number.parseInt(port, 10);
    const trimmedKey = sshPrivateKey.trim();

    if (!trimmedName || !trimmedHost || !trimmedUser || !trimmedKey) {
      toast.error('All fields except Port are required');
      return;
    }

    if (
      parsedPort !== undefined &&
      (Number.isNaN(parsedPort) || parsedPort < 1 || parsedPort > 65535)
    ) {
      toast.error('Port must be a valid number between 1 and 65535');
      return;
    }

    const payload = {
      name: trimmedName,
      host: trimmedHost,
      user: trimmedUser,
      port: parsedPort,
      ssh_private_key: trimmedKey,
    };

    const promise = createNode.mutateAsync(payload);

    toast.promise(promise, {
      loading: 'Probing connection and creating node...',
      success: 'Node record created successfully. Initiating server setup.',
      error: (err) => err.message || 'Failed to connect/create node.',
    });

    try {
      const data = await promise;
      void queryClient.invalidateQueries({ queryKey: ['nodes'] });
      navigate(`/nodes/${data.node.id}?type=setup`);
    } catch {}
  };

  return (
    <div>
      <div>
        <h3 className="font-semibold text-text">Connect Node</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Add a remote Docker host to your cluster. Slasha will connect via SSH
          and automatically provision the server.
        </p>
      </div>

      <div className="mt-6">
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
              required
              placeholder="-----BEGIN OPENSSH PRIVATE KEY-----&#10;..."
              className="min-h-[150px] font-mono text-xs"
              value={sshPrivateKey}
              onChange={(e) => setSshPrivateKey(e.target.value)}
            />
            <p className="text-xs text-text-tertiary">
              The private key used to authenticate. Slasha keeps this securely
              saved.
            </p>
          </div>

          <div className="flex items-center justify-end gap-2 pt-4">
            <Button
              variant="ghost"
              label="Cancel"
              type="button"
              onClick={() => navigate('/nodes')}
              isDisabled={createNode.isPending}
            />
            <Button
              type="submit"
              label="Connect Node"
              isLoading={createNode.isPending}
              isDisabled={
                createNode.isPending ||
                !name.trim() ||
                !host.trim() ||
                !user.trim() ||
                !sshPrivateKey.trim()
              }
            />
          </div>
        </form>
      </div>
    </div>
  );
}
