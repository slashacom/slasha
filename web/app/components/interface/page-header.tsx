import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type PageHeaderProps = {
  title: React.ReactNode;
  description?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
};

export function PageHeader(props: PageHeaderProps) {
  const { title, description, actions, className } = props;

  return (
    <HStack
      justifyContent="between"
      alignItems="start"
      className={cn('gap-4', className)}
    >
      <VStack space={2} className="min-w-0">
        <h3 className="text-balance font-semibold text-text">{title}</h3>
        {description ? (
          <p className="max-w-prose text-pretty text-sm text-text-secondary">
            {description}
          </p>
        ) : null}
      </VStack>
      {actions ? (
        <HStack space={2} alignItems="center" className="shrink-0">
          {actions}
        </HStack>
      ) : null}
    </HStack>
  );
}
