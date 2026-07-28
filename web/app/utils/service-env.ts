// Build the `${{ service.KEY }}` reference an app uses to consume a service
// variable. Centralised so the literal `${{ }}` escaping lives in one place.
export function serviceEnvReference(serviceName: string, key: string): string {
  return `\${{ ${serviceName}.${key} }}`;
}
