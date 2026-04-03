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
      'flex cursor-pointer disabled:cursor-not-allowed items-center focus:outline-none focus:ring-2 focus:ring-offset-2 gap-2 rounded-lg px-4 py-1.5 text-sm font-medium transition-all disabled:opacity-50',
      color === 'neutral' &&
        'bg-neutral-200 text-neutral-900 focus:ring-neutral-200 hover:bg-neutral-300',
      color === 'primary' &&
        'bg-black text-white focus:ring-black/20 focus:ring-offset-black/20  hover:opacity-80',
      color === 'success' &&
        'bg-green-500 text-white focus:ring-green-500/20 focus:ring-offset-green-500/20  hover:opacity-80',
      color === 'error' &&
        'bg-red-500 text-white focus:ring-red-500/20 focus:ring-offset-red-500/20  hover:opacity-80',
      variant === 'ghost' &&
        'bg-transparent disabled:bg-transparent text-neutral-900 hover:text-black focus:ring-neutral-200 hover:bg-neutral-200',
      variant === 'link' &&
        'bg-transparent disabled:bg-transparent text-neutral-900 hover:text-black focus:ring-neutral-200 hover:bg-neutral-200',
      variant === 'link' && color === 'success' && 'text-green-500',
      variant === 'link' && color === 'error' && 'text-red-500',
      !isInteractable && 'pointer-events-none',
      !label && 'p-1.5',
      variant === 'ghost' && color === 'neutral' && 'text-neutral-400',
      noOutline && 'focus:ring-0 focus:ring-offset-0',
      size === 'sm' && 'pl-1.5 pr-2 rounded-sm py-1.5 text-xs py-1',
      size === 'lg' && 'px-5 py-2.5 text-base',
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
