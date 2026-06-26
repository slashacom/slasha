// Pick the most representative variable to show in connection examples,
// preferring a full connection string when the service exposes one.
export function primaryEnvKey(keys: string[]): string {
  return keys.includes('DATABASE_URL')
    ? 'DATABASE_URL'
    : (keys[0] ?? 'DATABASE_URL');
}

// Build the `${{ service.KEY }}` reference an app uses to consume a service
// variable. Centralised so the literal `${{ }}` escaping lives in one place.
export function serviceEnvReference(serviceName: string, key: string): string {
  return `\${{ ${serviceName}.${key} }}`;
}
