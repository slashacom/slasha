import type { Service } from '~/models/service';

type ServiceKindBadgeProps = {
  service: Service;
};

export function ServiceKindBadge(props: ServiceKindBadgeProps) {
  const { service } = props;
  return (
    <span className="rounded bg-white/5 px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
      {service.kind} {service.version}
    </span>
  );
}
