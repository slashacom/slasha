import type { ComponentType } from 'react';
import { Button, type ButtonColor } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type EmptyPageSize = 'sm' | 'md' | 'lg';

type EmptyPageProps = {
  icon: ComponentType<{ className?: string }>;
  title: string;
  subtitle?: string;
  actionLabel?: string;
  actionIcon?: React.ReactNode;
  actionColor?: ButtonColor;
  actionTo?: string;
  onAction?: () => void;
  secondaryLabel?: string;
  secondaryTo?: string;
  onSecondaryAction?: () => void;
  size?: EmptyPageSize;
  bordered?: boolean;
  children?: React.ReactNode;
  className?: string;
};

const sizeClasses: Record<EmptyPageSize, string> = {
  sm: 'px-6 py-10',
  md: 'px-6 py-14',
  lg: 'px-6 py-20',
};

export function EmptyPage(props: EmptyPageProps) {
  const {
    icon: Icon,
    title,
    subtitle,
    actionLabel,
    actionIcon,
    actionColor = 'primary',
    actionTo,
    onAction,
    secondaryLabel,
    secondaryTo,
    onSecondaryAction,
    size = 'md',
    bordered = true,
    children,
    className,
  } = props;

  const hasAction = Boolean(actionLabel) && Boolean(actionTo || onAction);
  const hasSecondary =
    Boolean(secondaryLabel) && Boolean(secondaryTo || onSecondaryAction);

  return (
    <VStack
      alignItems="center"
      space={5}
      className={cn(
        'relative w-full justify-center overflow-hidden text-center',
        sizeClasses[size],
        bordered &&
          'rounded-lg border border-dashed border-border bg-surface/30',
        className
      )}
    >
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(hsl(0_0%_100%/6%)_1px,transparent_1px)] [background-size:18px_18px] [mask-image:radial-gradient(ellipse_60%_60%_at_50%_45%,#000,transparent)]"
      />

      <VStack alignItems="center" space={4} className="relative">
        <div className="flex size-11 items-center justify-center rounded-xl border border-border bg-surface text-text-secondary shadow-none">
          <Icon className="size-5" />
        </div>

        <VStack alignItems="center" space={1.5}>
          <p className="text-balance text-sm font-medium text-text">{title}</p>
          {subtitle ? (
            <p className="max-w-[380px] text-balance text-[13px] leading-relaxed text-text-tertiary">
              {subtitle}
            </p>
          ) : null}
        </VStack>
      </VStack>

      {children ? <div className="relative">{children}</div> : null}

      {hasAction || hasSecondary ? (
        <HStack space={2} className="relative">
          {hasAction ? (
            <Button
              to={actionTo}
              label={actionLabel}
              icon={actionIcon}
              color={actionColor}
              size="sm"
              onClick={onAction}
            />
          ) : null}
          {hasSecondary ? (
            <Button
              to={secondaryTo}
              label={secondaryLabel}
              color="neutral"
              size="sm"
              onClick={onSecondaryAction}
            />
          ) : null}
        </HStack>
      ) : null}
    </VStack>
  );
}
