import { Modal } from "./Modal";
import { LogViewer } from "@/components/LogViewer";

interface Props {
  scriptId: number;
  onClose: () => void;
}

export function ScriptLogsModal({ scriptId, onClose }: Props) {
  return (
    <Modal title="Script Output" onClose={onClose} wide>
      <div className="h-[500px]">
        <LogViewer entityType="script" entityId={scriptId} />
      </div>
    </Modal>
  );
}
