import { forwardRef } from 'react';
import { Link } from 'react-router';
import { Loader } from '~/components/icons/loader';
import { cn } from '~/utils/classname';

export type ButtonColor = 'neutral' | 'primary' | 'success' | 'error';

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  to?: string;
  label?: string;
  icon?: React.ReactNode;
  color?: ButtonColor;
  variant?: 'default' | 'link' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  isLoading?: boolean;
  isDisabled?: boolean;
  isInteractable?: boolean;
  target?: string;
  noOutline?: boolean;
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (props, ref) => {
    const {
      label,
      noOutline = false,
      to = '',
      icon,
      color = 'primary',
      variant = 'default',
      size = 'md',
      isLoading = false,
      isDisabled = false,
      isInteractable = true,
      onClick,
      type = 'button',
      className,
      target,
      ...rest
    } = props;

    const classes = cn(
      'inline-flex cursor-pointer items-center gap-1.5 rounded-md px-3 text-[13px] font-medium outline-none transition-colors disabled:cursor-not-allowed disabled:opacity-50',
      color === 'primary' &&
        variant === 'default' &&
        '!bg-white !text-bg !no-underline hover:!bg-white/90',
      color === 'neutral' &&
        variant === 'default' &&
        'border border-border bg-surface text-text hover:bg-white/5',
      color === 'success' &&
        variant === 'default' &&
        'bg-emerald-500 text-white hover:bg-emerald-500/90',
      color === 'error' &&
        variant === 'default' &&
        'bg-red-600 text-white hover:bg-red-600/90',
      variant === 'ghost' &&
        'bg-transparent text-text-secondary hover:bg-white/5 hover:text-text',
      variant === 'ghost' &&
        color === 'error' &&
        'text-red-500 hover:bg-red-500/10 hover:text-red-400',
      variant === 'link' &&
        'h-auto bg-transparent px-0 py-0 text-text-secondary hover:text-text',
      variant === 'link' && color === 'success' && 'text-emerald-400',
      variant === 'link' && color === 'error' && 'text-red-500',
      !isInteractable && 'pointer-events-none',
      !label && 'p-1.5',
      size === 'sm' && 'h-7 rounded px-2 text-[12px]',
      size === 'md' && 'h-8',
      size === 'lg' && 'h-10 px-4 text-[14px]',
      noOutline && 'focus:ring-0 focus:ring-offset-0',
      className
    );

    const loaderClasses = cn(
      'size-4 animate-spin',
      size === 'sm' && 'size-3',
      size === 'lg' && 'size-5'
    );

    if (to) {
      return (
        <Link to={to} className={classes} target={target}>
          {isLoading && <Loader className={loaderClasses} />}
          {!isLoading && (
            <>
              {icon}
              {label && label}
            </>
          )}
        </Link>
      );
    }

    return (
      <button
        ref={ref}
        type={type}
        onClick={onClick}
        disabled={isDisabled}
        className={classes}
        {...rest}
      >
        {isLoading && <Loader className={loaderClasses} />}
        {!isLoading && (
          <>
            {icon}
            {label && label}
          </>
        )}
      </button>
    );
  }
);

Button.displayName = 'Button';
