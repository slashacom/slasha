import { cn } from '~/utils/classname';

type AvatarSize = 'sm' | 'md' | 'lg';

type AvatarProps = {
  name: string;
  size?: AvatarSize;
  className?: string;
};

const lightColors = [
  'bg-blue-100 text-neutral-500',
  'bg-purple-100 text-purple-800',
  'bg-indigo-100 text-indigo-800',
  'bg-cyan-100 text-cyan-800',
  'bg-teal-100 text-teal-800',
  'bg-green-100 text-green-800',
  'bg-lime-100 text-lime-800',
  'bg-yellow-100 text-yellow-800',
  'bg-amber-100 text-amber-800',
  'bg-orange-100 text-orange-800',
  'bg-red-100 text-red-800',
];

function getConsistentColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash << 5) - hash + name.charCodeAt(i);
    hash = hash & hash;
  }
  return lightColors[Math.abs(hash) % lightColors.length];
}

export function Avatar(props: AvatarProps) {
  const { name, size = 'md', className } = props;

  const initial = name.charAt(0).toUpperCase();
  const colorClasses = getConsistentColor(name);

  return (
    <div
      className={cn(
        'flex items-center justify-center rounded-full',
        colorClasses,
        size === 'sm' && 'size-6 text-xs',
        size === 'md' && 'h-10 w-10 text-base',
        size === 'lg' && 'h-12 w-12 text-lg',
        className
      )}
    >
      {initial}
    </div>
  );
}
