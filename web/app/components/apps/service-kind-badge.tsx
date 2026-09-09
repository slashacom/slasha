import { Database, Layers, Zap, type LucideIcon } from 'lucide-react';
import type { Service, ServiceKind } from '~/models/service';
import { cn } from '~/utils/classname';

const KIND_ICONS: Record<ServiceKind, LucideIcon> = {
  PostgreSQL: Database,
  MySQL: Database,
  MongoDB: Layers,
  Redis: Zap,
};

type ServiceKindBadgeProps = {
  service: Service;
};

export function ServiceKindBadge(props: ServiceKindBadgeProps) {
  const { service } = props;
  return (
    <span className="whitespace-nowrap rounded bg-white/5 px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
      {service.kind} {service.version}
    </span>
  );
}

type ServiceKindIconProps = {
  kind: ServiceKind;
  className?: string;
};

export function ServiceKindIcon(props: ServiceKindIconProps) {
  const { kind, className } = props;
  const Icon = KIND_ICONS[kind];

  return (
    <span
      aria-hidden
      className={cn(
        'flex size-8 shrink-0 items-center justify-center rounded-lg border border-border bg-surface text-text-secondary',
        className
      )}
    >
      <Icon className="size-4" />
    </span>
  );
}
