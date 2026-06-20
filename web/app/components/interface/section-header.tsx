import type { LucideIcon } from 'lucide-react';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type SectionHeaderProps = {
  icon?: LucideIcon;
  title?: string;
  description?: string;
  actions?: React.ReactNode;
  className?: string;
};

export function SectionHeader(props: SectionHeaderProps) {
  const { icon: Icon, title, description, actions, className } = props;
  return (
    <HStack
      justifyContent="between"
      alignItems="center"
      className={cn(
        'h-16 shrink-0 gap-4 border-b border-border px-8',
        className
      )}
    >
      <HStack space={2} alignItems={description ? 'start' : 'center'}>
        {Icon ? (
          <Icon
            className={cn('size-4 text-text-tertiary', description && 'mt-0.5')}
          />
        ) : null}
        <VStack space={0.5}>
          {title ? (
            <h2 className="text-sm font-semibold text-text">{title}</h2>
          ) : null}
          {description ? (
            <p className="text-[12px] text-text-tertiary">{description}</p>
          ) : null}
        </VStack>
      </HStack>
      {actions ? (
        <HStack space={2} alignItems="center" className="shrink-0">
          {actions}
        </HStack>
      ) : null}
    </HStack>
  );
}
