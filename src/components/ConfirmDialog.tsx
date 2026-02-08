import { useEffect } from 'preact/hooks';

interface Props {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({ title, message, confirmLabel = 'Confirm', danger = false, onConfirm, onCancel }: Props) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onCancel]);

  const handleOverlayClick = (e: Event) => {
    if ((e.target as HTMLElement).classList.contains('command-form-overlay')) {
      onCancel();
    }
  };

  return (
    <div class="command-form-overlay" onClick={handleOverlayClick}>
      <div class="confirm-dialog">
        <h3>{title}</h3>
        <p>{message}</p>
        <div class="form-actions">
          <button type="button" class="btn-secondary" onClick={onCancel}>
            Cancel
          </button>
          <button
            type="button"
            class={danger ? 'btn-danger' : 'btn-primary'}
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
