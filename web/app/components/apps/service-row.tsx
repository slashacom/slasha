import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { Eye, MoreHorizontal, Trash2 } from 'lucide-react';
import type { Service } from '~/models/service';
import { useDeleteService } from '~/queries/services';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '~/components/interface/dropdown-menu';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { formatRelativeTime } from '~/utils/format';
import { toast } from 'sonner';

type ServiceRowProps = {
  service: Service;
  appSlug: string;
};

export function ServiceRow(props: ServiceRowProps) {
  const { service, appSlug } = props;
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const deleteService = useDeleteService();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const goToDetail = () => {
    navigate(`/apps/${appSlug}/services/${service.id}`);
  };

  const handleDelete = async () => {
    try {
      await deleteService.mutateAsync({ appSlug, serviceId: service.id });
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'services'] });
      setShowDeleteConfirm(false);
    } catch (e) {
      toast.error('Failed to delete service: ' + e);
    }
  };

  return (
    <>
      <div
        onClick={goToDetail}
        className="group grid cursor-pointer grid-cols-[1fr_auto] items-center gap-4 px-8 py-4 transition-colors hover:bg-white/[0.02]"
      >
        <VStack space={1.5}>
          <HStack space={3}>
            <span className="font-mono text-[13px] font-semibold text-text transition-colors group-hover:text-primary">
              {service.name}
            </span>
            <span className="rounded bg-white/5 px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
              {service.kind} {service.version}
            </span>
            <StatusBadge status={service.status} />
          </HStack>
          <HStack space={3}>
            <span className="text-[11px] text-text-tertiary">
              slasha-svc-{service.id.slice(0, 8)}
            </span>
            <span className="text-[11px] text-text-tertiary">
              Created {formatRelativeTime(service.created_at)}
            </span>
          </HStack>
        </VStack>

        <div onClick={(e) => e.stopPropagation()}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                aria-label="Service actions"
                className="flex size-7 items-center justify-center rounded-md text-text-tertiary opacity-60 transition-all hover:bg-white/5 hover:text-text group-hover:opacity-100 data-[state=open]:bg-white/5 data-[state=open]:text-text data-[state=open]:opacity-100"
              >
                <MoreHorizontal className="size-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={goToDetail}>
                <Eye className="size-3.5" />
                View service
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                onClick={() => setShowDeleteConfirm(true)}
              >
                <Trash2 className="size-3.5" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Service"
        description={`Are you sure you want to delete ${service.name}? All underlying data will be permanently destroyed.`}
        confirmLabel="Delete Service"
        onConfirm={handleDelete}
      />
    </>
  );
}
