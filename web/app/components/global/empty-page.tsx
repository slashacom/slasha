import type { ReactNode } from 'react';
import { Button, type ButtonColor } from '~/components/interface/button';

type EmptyPageProps = {
  icon: ReactNode;
  message: string;
  buttonIcon: ReactNode;
  buttonLabel: string;
  buttonColor?: ButtonColor;
  onButtonClick?: () => void;
};

export function EmptyPage(props: EmptyPageProps) {
  const {
    icon,
    message,
    buttonIcon,
    buttonLabel,
    buttonColor = 'neutral',
    onButtonClick,
  } = props;

  return (
    <div className="flex flex-col items-center justify-center h-full flex-grow">
      {icon}
      <p className="text-sm text-neutral-500 mb-5">{message}</p>
      {onButtonClick && (
        <Button
          icon={buttonIcon}
          color={buttonColor}
          label={buttonLabel}
          onClick={onButtonClick}
        />
      )}
    </div>
  );
}
