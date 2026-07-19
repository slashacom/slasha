import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Globe,
  Plus,
  RefreshCw,
  Trash2,
  Loader2,
  ExternalLink,
} from 'lucide-react';
import { toast } from 'sonner';

import {
  getAppDomainsHealthOptions,
  getAppDomainsOptions,
  useAddAppDomain,
  useDeleteAppDomain,
} from '~/queries/apps';
import type { DomainHealth } from '~/models/domain-health';
import { Button } from '~/components/interface/button';
import { DomainHealthBadge } from '~/components/apps/domain-health-badge';
import { DomainHealthDetail } from '~/components/apps/domain-health-detail';
import { EmptyPage } from '~/components/global/empty-page';
import { Input } from '~/components/interface/input';
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
  const {
    data: healthData,
    isLoading: healthLoading,
    isFetching: healthFetching,
    refetch: refetchHealth,
  } = useQuery(getAppDomainsHealthOptions(appSlug));
  const addDomain = useAddAppDomain();
  const deleteDomain = useDeleteAppDomain();

  const [newDomain, setNewDomain] = useState('');

  const handleAdd = async (e: React.SubmitEvent) => {
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
      toast.error(e?.message || 'Failed to add domain');
    }
  };

  const handleDelete = async (domainId: string) => {
    try {
      await deleteDomain.mutateAsync({ appSlug, domainId });
      toast.success('Domain removed successfully');
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'domains'] });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to remove domain');
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
  const healthByDomain = new Map<string, DomainHealth>(
    (healthData?.health ?? []).map((entry) => [entry.domain, entry])
  );
  const serverIps = deriveServerIps(healthData?.health ?? []);

  return (
    <VStack space={6}>
      <div className="overflow-hidden rounded-xl border border-border bg-surface/50 shadow-sm backdrop-blur-sm">
        <div className="flex items-center justify-between border-b border-border bg-surface/50 px-6 py-5">
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
          {domains.length > 0 && (
            <button
              type="button"
              onClick={() => refetchHealth()}
              disabled={healthFetching}
              className="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[12px] text-text-tertiary transition-colors hover:bg-white/5 hover:text-text disabled:opacity-50"
            >
              <RefreshCw
                className={cn('size-3.5', healthFetching && 'animate-spin')}
              />
              Recheck
            </button>
          )}
        </div>

        <div className="p-6">
          <form onSubmit={handleAdd} className="mb-6">
            <HStack space={2}>
              <div className="relative flex-1">
                <Globe className="absolute left-3 top-1/2 z-10 size-3.5 -translate-y-1/2 text-text-tertiary" />
                <Input
                  placeholder="example.com"
                  value={newDomain}
                  onChange={(e) => setNewDomain(e.target.value)}
                  className="pl-9 text-[13px]"
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
              {serverIps.length ? (
                <>
                  Point your domain’s A record to{' '}
                  <span className="font-mono text-text-secondary">
                    {serverIps.join(', ')}
                  </span>
                  . HTTPS is provisioned automatically.
                </>
              ) : (
                <>
                  Ensure your domain’s A or CNAME record points to this server’s
                  IP address. HTTPS is provisioned automatically.
                </>
              )}
            </p>
          </form>

          {domains.length === 0 ? (
            <EmptyPage
              dashed
              icon={Globe}
              title="No custom domains added yet."
            />
          ) : (
            <div className="divide-y divide-border rounded-lg border border-border bg-surface/20">
              {domains.map((domain) => {
                const health = healthByDomain.get(domain.domain);

                return (
                  <div key={domain.id} className="px-4 py-3.5">
                    <HStack justifyContent="between" alignItems="start">
                      <VStack space={0} className="min-w-0">
                        <HStack space={3}>
                          <Globe className="size-3.5 shrink-0 text-text-tertiary" />
                          <span className="truncate text-[13px] font-medium text-text">
                            {domain.domain}
                          </span>
                          <a
                            href={`https://${domain.domain}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex items-center gap-1 text-[11px] text-text-tertiary transition-colors hover:text-text"
                          >
                            Open
                            <ExternalLink className="size-3" />
                          </a>
                        </HStack>
                      </VStack>
                      <HStack space={2} className="shrink-0">
                        <DomainHealthBadge
                          status={
                            healthLoading && !health
                              ? 'checking'
                              : (health?.status ?? 'unknown')
                          }
                        />
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
                    </HStack>
                    {health && <DomainHealthDetail health={health} />}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </VStack>
  );
}

function deriveServerIps(health: DomainHealth[]): string[] {
  for (const entry of health) {
    if (entry.dns.expected_ips.length) {
      return entry.dns.expected_ips;
    }
  }

  return [];
}
