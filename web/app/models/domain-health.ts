export type HealthStatus = 'healthy' | 'pending' | 'error' | 'unknown';

export type DnsStatus =
  | 'ok'
  | 'proxied'
  | 'mismatch'
  | 'unresolved'
  | 'unknown';

export type TlsStatus =
  | 'active'
  | 'pending'
  | 'expired'
  | 'unreachable'
  | 'unknown';

export type DnsHealth = {
  status: DnsStatus;
  resolved_ips: string[];
  expected_ips: string[];
  proxy: string | null;
};

export type TlsHealth = {
  status: TlsStatus;
  issuer: string | null;
  expires_at: string | null;
  days_until_expiry: number | null;
};

export type DomainHealth = {
  domain: string;
  status: HealthStatus;
  dns: DnsHealth;
  tls: TlsHealth;
};
