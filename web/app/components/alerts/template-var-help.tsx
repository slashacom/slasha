const TEMPLATE_VARS = [
  ['{{detail}}', 'System-generated alert description'],
  ['{{value}}', 'Current metric value'],
  ['{{notification_status}}', 'triggered | renotified | resolved'],
  ['{{alert_kind}}', 'Alert kind (server_cpu, app_memory, etc.)'],
];

export function TemplateVarHelp() {
  return (
    <div className="rounded-md border border-border bg-bg/40 px-3 py-2">
      <p className="text-[11px] text-text-tertiary">Available variables:</p>
      <div className="mt-2 grid gap-1">
        {TEMPLATE_VARS.map(([name, description]) => (
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
