import type { DomainHealth } from '~/models/domain-health';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type Tone = 'ok' | 'warn' | 'error' | 'neutral';

type Line = {
  tone: Tone;
  label: string;
  detail: string;
};

type DomainHealthDetailProps = {
  health: DomainHealth;
};

const DOT_STYLES: Record<Tone, string> = {
  ok: 'bg-emerald-400',
  warn: 'bg-amber-400',
  error: 'bg-red-400',
  neutral: 'bg-text-tertiary',
};

function dnsLine(health: DomainHealth): Line {
  const { status, resolved_ips, expected_ips } = health.dns;

  if (status === 'ok') {
    return {
      tone: 'ok',
      label: 'DNS configured',
      detail: `Resolves to ${resolved_ips.join(', ')}`,
    };
  }

  if (status === 'mismatch') {
    return {
      tone: 'error',
      label: 'DNS misconfigured',
      detail: `Resolves to ${resolved_ips.join(', ')} — expected ${expected_ips.join(', ')}`,
    };
  }

  if (status === 'unresolved') {
    return {
      tone: 'error',
      label: 'DNS not resolving',
      detail: expected_ips.length
        ? `No record found. Point an A record to ${expected_ips.join(', ')}`
        : 'No A or AAAA record found for this domain',
    };
  }

  return {
    tone: 'neutral',
    label: 'DNS',
    detail: 'Could not determine this server’s address',
  };
}

function tlsLine(health: DomainHealth): Line {
  const { status, issuer, days_until_expiry } = health.tls;

  if (status === 'active') {
    const parts: string[] = [];
    if (issuer) {
      parts.push(`Issued by ${issuer}`);
    }
    if (days_until_expiry != null) {
      parts.push(`valid ${days_until_expiry} more days`);
    }

    return {
      tone: 'ok',
      label: 'TLS certificate active',
      detail: parts.join(' · ') || 'Serving a valid certificate',
    };
  }

  if (status === 'pending') {
    return {
      tone: 'warn',
      label: 'TLS certificate provisioning',
      detail: 'Awaiting a certificate — verify DNS points to this server',
    };
  }

  if (status === 'expired') {
    return {
      tone: 'error',
      label: 'TLS certificate expired',
      detail: issuer
        ? `Issued by ${issuer}`
        : 'The served certificate is no longer valid',
    };
  }

  if (status === 'unreachable') {
    return {
      tone: 'error',
      label: 'No HTTPS response',
      detail: 'The server is not answering on port 443',
    };
  }

  return {
    tone: 'neutral',
    label: 'TLS',
    detail: 'Could not verify the certificate',
  };
}

function HealthLine(props: { line: Line }) {
  const { line } = props;

  return (
    <HStack space={2} alignItems="start">
      <span
        className={cn(
          'mt-1.5 size-1.5 shrink-0 rounded-full',
          DOT_STYLES[line.tone]
        )}
      />
      <div className="min-w-0">
        <span className="text-[12px] font-medium text-text-secondary">
          {line.label}
        </span>
        <span className="ml-1.5 text-[12px] text-text-tertiary">
          {line.detail}
        </span>
      </div>
    </HStack>
  );
}

export function DomainHealthDetail(props: DomainHealthDetailProps) {
  const { health } = props;

  return (
    <VStack space={1.5} className="mt-3 border-t border-border/60 pt-3">
      <HealthLine line={dnsLine(health)} />
      <HealthLine line={tlsLine(health)} />
    </VStack>
  );
}
