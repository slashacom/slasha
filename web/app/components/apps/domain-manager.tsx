import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Globe, Plus, Trash2, Loader2, ExternalLink } from 'lucide-react';
import { toast } from 'sonner';

import {
  getAppDomainsOptions,
  useAddAppDomain,
  useDeleteAppDomain,
} from '~/queries/apps';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type DomainManagerProps = {
  appSlug: string;
};

export function DomainManager(props: DomainManagerProps) {
  const { appSlug } = props;
  const queryClient = useQueryClient();
  const { data: domainsData, isLoading: domainsLoading } = useQuery(
    getAppDomainsOptions(appSlug)
  );
  const addDomain = useAddAppDomain();
  const deleteDomain = useDeleteAppDomain();

  const [newDomain, setNewDomain] = useState('');

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newDomain.trim()) {
      return;
    }

    try {
      await addDomain.mutateAsync({ appSlug, domain: newDomain.trim() });
      setNewDomain('');
      toast.success('Domain added successfully');
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'domains'] });
    } catch (e: any) {
      toast.error(e.response?.data?.error || 'Failed to add domain');
    }
  };

  const handleDelete = async (domainId: string) => {
    try {
      await deleteDomain.mutateAsync({ appSlug, domainId });
      toast.success('Domain removed successfully');
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'domains'] });
    } catch (e: any) {
      toast.error(e.response?.data?.error || 'Failed to remove domain');
    }
  };

  if (domainsLoading) {
    return (
      <div className="flex h-32 items-center justify-center text-text-tertiary">
        <Loader2 className="size-5 animate-spin" />
        <span className="ml-2">Loading domains...</span>
      </div>
    );
  }

  const domains = domainsData?.domains ?? [];

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div className="border-b border-border bg-surface/50 px-6 py-5">
          <HStack space={3}>
            <div className="rounded-lg bg-white/5 p-2 text-text-secondary">
              <Globe className="size-5" />
            </div>
            <div>
              <h3 className="text-[15px] font-semibold text-text">Domains</h3>
              <p className="mt-0.5 text-[13px] text-text-tertiary">
                Manage custom domains for your application.
              </p>
            </div>
          </HStack>
        </div>

        <div className="p-6">
          <form onSubmit={handleAdd} className="mb-6">
            <HStack space={2}>
              <div className="relative flex-1">
                <Globe className="absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-text-tertiary" />
                <input
                  type="text"
                  placeholder="example.com"
                  value={newDomain}
                  onChange={(e) => setNewDomain(e.target.value)}
                  className="h-9 w-full rounded-md border border-border bg-surface px-9 text-[13px] text-text outline-none focus:border-white/20"
                />
              </div>
              <Button
                type="submit"
                label="Add Domain"
                icon={<Plus className="size-3.5" />}
                isLoading={addDomain.isPending}
                size="sm"
              />
            </HStack>
            <p className="mt-2 text-[11px] text-text-tertiary">
              Ensure your domain's A or CNAME record points to this server's IP
              address.
            </p>
          </form>

          {domains.length === 0 ? (
            <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-8">
              <p className="text-[13px] text-text-tertiary">
                No custom domains added yet.
              </p>
            </div>
          ) : (
            <div className="divide-y divide-border rounded-lg border border-border bg-surface/20">
              {domains.map((domain) => (
                <div key={domain.id} className="px-4 py-3">
                  <HStack justifyContent="between">
                    <HStack space={3}>
                      <Globe className="size-3.5 text-text-tertiary" />
                      <span className="text-[13px] font-medium text-text">
                        {domain.domain}
                      </span>
                      <a
                        href={`http://${domain.domain}`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-text-tertiary transition-colors hover:text-text"
                      >
                        <ExternalLink className="size-3" />
                      </a>
                    </HStack>
                    <Button
                      variant="ghost"
                      color="error"
                      icon={<Trash2 className="size-3.5" />}
                      onClick={() => handleDelete(domain.id)}
                      isLoading={
                        deleteDomain.isPending &&
                        deleteDomain.variables?.domainId === domain.id
                      }
                      className="size-8"
                    />
                  </HStack>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </VStack>
  );
}
