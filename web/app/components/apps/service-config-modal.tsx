import type { Service } from '~/models/service';
import {
  Dialog,
  DialogContent,
} from '~/components/interface/dialog';
import { ServiceEnvEditor } from '~/components/apps/service-env-editor';

type ServiceConfigModalProps = {
  appSlug: string;
  service: Service;
  onClose: () => void;
};

export function ServiceConfigModal(props: ServiceConfigModalProps) {
  const { appSlug, service, onClose } = props;
  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl border-none bg-transparent p-0 shadow-none">
        <ServiceEnvEditor
          appSlug={appSlug}
          serviceId={service.id}
          serviceName={service.name}
          readOnly={true}
          onCancel={onClose}
        />
      </DialogContent>
    </Dialog>
  );
}
