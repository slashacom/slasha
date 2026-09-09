import type { ServiceKind } from '~/models/service';

// Build the `${{ service.KEY }}` reference an app uses to consume a service
// variable. Centralised so the literal `${{ }}` escaping lives in one place.
export function serviceEnvReference(serviceName: string, key: string): string {
  return `\${{ ${serviceName}.${key} }}`;
}

export function serviceProxyCommand(
  appSlug: string,
  serviceName: string
): string {
  return `slasha proxy --app ${appSlug} ${serviceName}`;
}

// Mirrors `ServiceKind::secret_env_keys` in slasha-db, which decides which
// variables the backend generates a random password for.
const SERVICE_SECRET_KEYS: Record<ServiceKind, readonly string[]> = {
  PostgreSQL: ['POSTGRES_PASSWORD'],
  MySQL: ['MYSQL_ROOT_PASSWORD', 'MYSQL_PASSWORD'],
  MongoDB: ['MONGO_INITDB_ROOT_PASSWORD'],
  Redis: ['REDIS_PASSWORD'],
};

export function isServiceSecret(kind: ServiceKind, key: string): boolean {
  if (SERVICE_SECRET_KEYS[kind].includes(key)) {
    return true;
  }

  return /password|secret|token/i.test(key);
}
