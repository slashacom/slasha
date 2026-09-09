import type { LucideIcon } from 'lucide-react';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type IncidentCardProps = {
  icon: LucideIcon;
  title: string;
  children: React.ReactNode;
};

export function IncidentCard(props: IncidentCardProps) {
  const { icon: Icon, title, children } = props;

  return (
    <div className="flex min-w-0 flex-col rounded-lg border border-border bg-surface/30">
      <HStack space={1.5} className="border-b border-border px-4 py-2.5">
        <Icon className="size-3.5 text-text-tertiary" />
        <span className="text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
          {title}
        </span>
      </HStack>
      <div className="flex-1 px-4 py-3.5">{children}</div>
    </div>
  );
}

type IncidentRowProps = {
  label: string;
  children: React.ReactNode;
};

export function IncidentRow(props: IncidentRowProps) {
  const { label, children } = props;

  return (
    <HStack justifyContent="between" className="min-w-0 gap-3">
      <span className="shrink-0 text-[12px] text-text-tertiary">{label}</span>
      <span className="min-w-0 truncate text-[13px] tabular-nums text-text">
        {children}
      </span>
    </HStack>
  );
}

type IncidentTimelineProps = {
  openedAt: string;
  resolvedAt: string | null;
  duration: string;
};

export function IncidentTimeline(props: IncidentTimelineProps) {
  const { openedAt, resolvedAt, duration } = props;
  const isOpen = !resolvedAt;

  return (
    <VStack space={0} className="relative">
      <span
        aria-hidden
        className="absolute left-[3px] top-[9px] bottom-[9px] w-px bg-border"
      />

      <TimelineStep label="Opened" value={openedAt} />

      <HStack space={3} className="py-1.5 pl-[19px]">
        <span className="text-[12px] text-text-tertiary">
          {isOpen ? `${duration} and counting` : duration}
        </span>
      </HStack>

      <TimelineStep
        label={isOpen ? 'Unresolved' : 'Resolved'}
        value={resolvedAt ?? 'Still open'}
        isPending={isOpen}
      />
    </VStack>
  );
}

type TimelineStepProps = {
  label: string;
  value: string;
  isPending?: boolean;
};

function TimelineStep(props: TimelineStepProps) {
  const { label, value, isPending = false } = props;

  return (
    <HStack justifyContent="between" className="relative min-w-0 gap-3">
      <HStack space={3} className="min-w-0">
        <span
          aria-hidden
          className={cn(
            'size-[7px] shrink-0 rounded-full',
            isPending ? 'border border-text-tertiary' : 'bg-text-tertiary'
          )}
        />
        <span className="text-[12px] text-text-tertiary">{label}</span>
      </HStack>
      <span className="min-w-0 truncate text-[13px] tabular-nums text-text">
        {value}
      </span>
    </HStack>
  );
}

type IncidentMetaProps = {
  children: React.ReactNode;
};

export function IncidentMeta(props: IncidentMetaProps) {
  const { children } = props;

  return <div className="grid gap-3 md:grid-cols-3">{children}</div>;
}
