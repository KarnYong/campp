interface DebugMenuProps {
  onClose: () => void;
  onOpenRuntimeFolder: () => void;
  onOpenDownloadFolder: () => void;
  onResetInstallation: () => void;
}

export function DebugMenu({ onClose, onOpenRuntimeFolder, onOpenDownloadFolder, onResetInstallation }: DebugMenuProps) {
  return (
    <div className="debug-menu">
      <div className="debug-menu-header">
        <span>Debug Menu</span>
        <button className="debug-menu-close" onClick={onClose}>
          ×
        </button>
      </div>
      <div className="debug-menu-body">
        <button className="debug-menu-btn" onClick={onOpenRuntimeFolder}>
          Open Runtime Folder
        </button>
        <button className="debug-menu-btn" onClick={onOpenDownloadFolder}>
          View Download Folder
        </button>
        <button className="debug-menu-btn" onClick={onResetInstallation}>
          Reset Installation
        </button>
      </div>
    </div>
  );
}
