import { useState } from 'react';
import { toast } from 'sonner';
import { FormActions } from '~/components/interface/form-actions';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { PasswordInput } from '~/components/interface/password-input';
import { Switch } from '~/components/interface/switch';
import type { S3Storage } from '~/models/s3-storage';

export type S3StorageFormData = {
  name: string;
  endpoint: string;
  bucket: string;
  region: string;
  access_key_id: string;
  secret_access_key?: string;
  force_path_style: boolean;
};

type S3StorageFormProps = {
  storage?: S3Storage;
  onSubmit: (data: S3StorageFormData) => void;
  onCancel: () => void;
  isPending: boolean;
  submitLabel: string;
};

export function S3StorageForm(props: S3StorageFormProps) {
  const { storage, onSubmit, onCancel, isPending, submitLabel } = props;

  const [name, setName] = useState(storage?.name ?? '');
  const [endpoint, setEndpoint] = useState(storage?.endpoint ?? '');
  const [bucket, setBucket] = useState(storage?.bucket ?? '');
  const [region, setRegion] = useState(storage?.region ?? 'auto');
  const [accessKeyId, setAccessKeyId] = useState(storage?.access_key_id ?? '');
  const [secretAccessKey, setSecretAccessKey] = useState('');
  const [forcePathStyle, setForcePathStyle] = useState(
    storage?.force_path_style ?? false
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!storage && !secretAccessKey.trim()) {
      toast.error('Secret access key is required');
      return;
    }

    onSubmit({
      name: name.trim(),
      endpoint: endpoint.trim(),
      bucket: bucket.trim(),
      region: region.trim() || 'auto',
      access_key_id: accessKeyId.trim(),
      secret_access_key: secretAccessKey.trim() || undefined,
      force_path_style: forcePathStyle,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="w-full max-w-xl">
      <div className="space-y-5">
        <div className="space-y-1.5">
          <Label
            htmlFor="name"
            className="text-[13px] font-medium text-text-secondary"
          >
            Storage Name
          </Label>
          <Input
            id="name"
            name="name"
            required
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Production Backups"
            className="h-10"
          />
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="endpoint"
            className="text-[13px] font-medium text-text-secondary"
          >
            Endpoint URL
          </Label>
          <Input
            id="endpoint"
            name="endpoint"
            required
            value={endpoint}
            onChange={(e) => setEndpoint(e.target.value)}
            placeholder="e.g. https://s3.us-east-1.amazonaws.com"
            className="h-10 font-mono text-xs"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-1.5">
            <Label
              htmlFor="bucket"
              className="text-[13px] font-medium text-text-secondary"
            >
              Bucket Name
            </Label>
            <Input
              id="bucket"
              name="bucket"
              required
              value={bucket}
              onChange={(e) => setBucket(e.target.value)}
              placeholder="e.g. slashacom-backups"
              className="h-10 font-mono text-xs"
            />
          </div>

          <div className="space-y-1.5">
            <Label
              htmlFor="region"
              className="text-[13px] font-medium text-text-secondary"
            >
              Region
            </Label>
            <Input
              id="region"
              name="region"
              value={region}
              onChange={(e) => setRegion(e.target.value)}
              placeholder="auto"
              className="h-10 font-mono text-xs"
            />
          </div>
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="access_key_id"
            className="text-[13px] font-medium text-text-secondary"
          >
            Access Key ID
          </Label>
          <Input
            id="access_key_id"
            name="access_key_id"
            required
            value={accessKeyId}
            onChange={(e) => setAccessKeyId(e.target.value)}
            placeholder="e.g. AKIAIOSFODNN7EXAMPLE"
            className="h-10 font-mono text-xs"
          />
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="secret_access_key"
            className="text-[13px] font-medium text-text-secondary"
          >
            Secret Access Key
          </Label>
          <PasswordInput
            id="secret_access_key"
            name="secret_access_key"
            required={!storage}
            value={secretAccessKey}
            onChange={(e) => setSecretAccessKey(e.target.value)}
            placeholder={
              storage
                ? 'Leave empty to keep existing secret key'
                : 'e.g. wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY'
            }
            className="h-10 font-mono text-xs"
          />
        </div>

        <div className="flex items-start justify-between rounded-lg border border-border bg-surface/30 p-4">
          <div className="space-y-0.5 pr-4">
            <Label
              htmlFor="force_path_style"
              className="text-[13px] font-medium text-text"
            >
              Force Path-Style Addressing
            </Label>
            <p className="text-[12px] text-text-tertiary">
              Use path-style URLs (
              <code className="text-text-secondary">endpoint/bucket</code>)
              instead of virtual-hosted (
              <code className="text-text-secondary">bucket.endpoint</code>).
              Required for MinIO, Ceph, and custom local S3 gateways.
            </p>
          </div>
          <Switch
            id="force_path_style"
            checked={forcePathStyle}
            onCheckedChange={setForcePathStyle}
          />
        </div>

        <FormActions
          submitLabel={submitLabel}
          onCancel={onCancel}
          isPending={isPending}
          className="pt-2"
        />
      </div>
    </form>
  );
}
