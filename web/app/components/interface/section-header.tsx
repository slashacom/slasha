import type { LucideIcon } from 'lucide-react';
import { HStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type SectionHeaderProps = {
  icon: LucideIcon;
  title: string;
  badge?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
};

export function SectionHeader(props: SectionHeaderProps) {
  const { icon: Icon, title, badge, actions, className } = props;
  return (
    <HStack
      justifyContent="between"
      className={cn('border-b border-border px-8 py-4', className)}
    >
      <HStack space={2}>
        <Icon className="size-4 text-text-tertiary" />
        <h2 className="text-sm font-semibold text-text">{title}</h2>
        {badge}
      </HStack>
      {actions ? <HStack space={2}>{actions}</HStack> : null}
    </HStack>
  );
}
