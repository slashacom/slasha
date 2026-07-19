const ENV_VARS = [
  ['SLASHA_ALERT_DETAIL', 'System-generated alert description'],
  ['SLASHA_ALERT_VALUE', 'Current value'],
  ['SLASHA_ALERT_KIND', 'Alert kind (node_cpu, app_memory, etc.)'],
  ['SLASHA_ALERT_RULE_NAME', 'Rule name'],
  ['SLASHA_ALERT_STATUS', 'triggered | renotified | resolved'],
];

export function ShellCommandEnvHelp() {
  return (
    <div className="rounded-md border border-border bg-bg/40 px-3 py-2">
      <p className="text-[11px] text-text-tertiary">
        Runs with <code className="font-mono text-text">sh -lc</code> and these
        envs:
      </p>
      <div className="mt-2 grid gap-1">
        {ENV_VARS.map(([name, description]) => (
          <div
            key={name}
            className="flex items-baseline justify-between gap-3 text-[11px]"
          >
            <code className="font-mono text-text-secondary">{name}</code>
            <span className="text-text-tertiary">{description}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
