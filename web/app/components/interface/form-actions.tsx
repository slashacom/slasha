import { Button } from '~/components/interface/button';
import { HStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';

type FormActionsProps = {
  submitLabel: string;
  onSubmit?: () => void;
  onCancel?: () => void;
  cancelTo?: string;
  cancelLabel?: string;
  isPending?: boolean;
  isDisabled?: boolean;
  className?: string;
};

export function FormActions(props: FormActionsProps) {
  const {
    submitLabel,
    onSubmit,
    onCancel,
    cancelTo,
    cancelLabel = 'Cancel',
    isPending = false,
    isDisabled = false,
    className,
  } = props;

  const hasCancel = Boolean(onCancel || cancelTo);

  return (
    <HStack space={2} className={cn('pt-2', className)}>
      <Button
        type={onSubmit ? 'button' : 'submit'}
        label={submitLabel}
        onClick={onSubmit}
        isLoading={isPending}
        isDisabled={isPending || isDisabled}
      />
      {hasCancel ? (
        <Button
          type="button"
          to={cancelTo}
          label={cancelLabel}
          variant="ghost"
          onClick={onCancel}
          isDisabled={isPending}
        />
      ) : null}
    </HStack>
  );
}
