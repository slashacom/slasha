import type { LucideIcon } from 'lucide-react';
import { Button, type ButtonColor } from '~/components/interface/button';
import { VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type EmptyPageProps = {
  icon: LucideIcon;
  title: string;
  subtitle?: string;
  actionLabel?: string;
  actionIcon?: React.ReactNode;
  actionColor?: ButtonColor;
  onAction?: () => void;
  dashed?: boolean;
  className?: string;
};

export function EmptyPage(props: EmptyPageProps) {
  const {
    icon: Icon,
    title,
    subtitle,
    actionLabel,
    actionIcon,
    actionColor = 'neutral',
    onAction,
    dashed = false,
    className,
  } = props;

  return (
    <VStack
      alignItems="center"
      space={4}
      className={cn(
        'justify-center px-6 py-12 text-center',
        dashed && 'rounded-lg border border-dashed border-border',
        className
      )}
    >
      <div className="rounded-full border border-border p-4">
        <Icon className="size-8 text-text-tertiary" />
      </div>
      <VStack alignItems="center" space={1}>
        <p className="text-sm font-medium text-text">{title}</p>
        {subtitle && (
          <p className="max-w-[280px] text-xs text-text-tertiary">{subtitle}</p>
        )}
      </VStack>
      {onAction && actionLabel && (
        <Button
          label={actionLabel}
          icon={actionIcon}
          color={actionColor}
          size="sm"
          onClick={onAction}
        />
      )}
    </VStack>
  );
}
