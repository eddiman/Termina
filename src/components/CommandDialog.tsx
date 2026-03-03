import { useState, useEffect } from 'preact/hooks';
import { useComputed } from '@preact/signals';
import { api } from '../lib/api';
import {
  commands,
  statuses,
  healthStatuses,
  selectedCommand,
  editingCommand,
  isFormOpen,
  type ProcessStatus,
} from '../lib/store';
import { LogViewer } from './LogViewer';
import { ConfirmDialog } from './ConfirmDialog';

export function CommandDialog() {
  const id = selectedCommand.value;
  const command = commands.value.find(c => c.id === id);
  const [confirmAction, setConfirmAction] = useState<'delete' | 'restart' | null>(null);
  const [isRestarting, setIsRestarting] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') selectedCommand.value = null;
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  if (!command || !id) return null;

  const statusSignal = useComputed(() => statuses.value[id] ?? { type: 'Stopped' } as ProcessStatus);
  const status = statusSignal.value;
  const isRunning = status.type === 'Running';
  const isOneTime = command.command_type === 'OneTime';
  const healthStatus = !isOneTime && command.health_check_url
    ? healthStatuses.value[id]
    : undefined;

  const getStatusClass = (s: ProcessStatus): string => {
    if (s.type === 'Running') return 'running';
    if (s.type === 'Error') return 'error';
    if (s.type === 'Exited') return s.code === 0 ? 'stopped' : 'error';
    return 'stopped';
  };

  const getStatusText = (s: ProcessStatus): string => {
    if (s.type === 'Running') return 'Running';
    if (s.type === 'Error') return 'Error';
    if (s.type === 'Exited') {
      if (s.code === null) return 'Terminated';
      if (s.code === 0) return 'Exited (0)';
      return `Exited (${s.code})`;
    }
    return 'Stopped';
  };

  const handleToggle = async () => {
    try {
      if (isRunning) {
        await api.stopCommand(id);
        statuses.value = { ...statuses.value, [id]: { type: 'Stopped' } };
      } else {
        await api.startCommand(id);
        statuses.value = { ...statuses.value, [id]: { type: 'Running' } };
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      statuses.value = { ...statuses.value, [id]: { type: 'Error', message: errorMsg } };
    }
  };

  const doRestart = async () => {
    setConfirmAction(null);
    setIsRestarting(true);
    try {
      await api.restartCommand(id);
      statuses.value = { ...statuses.value, [id]: { type: 'Running' } };
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      statuses.value = { ...statuses.value, [id]: { type: 'Error', message: errorMsg } };
    } finally {
      setIsRestarting(false);
    }
  };

  const handleEdit = () => {
    editingCommand.value = command;
    isFormOpen.value = true;
  };

  const handleDelete = () => {
    setConfirmAction('delete');
  };

  const doDelete = async () => {
    setConfirmAction(null);
    try {
      if (isRunning) {
        await api.stopCommand(id);
      }
      await api.deleteCommand(id);
      commands.value = commands.value.filter(c => c.id !== id);
      const newStatuses = { ...statuses.value };
      delete newStatuses[id];
      statuses.value = newStatuses;
      selectedCommand.value = null;
    } catch (err) {
      console.error('Failed to delete command:', err);
    }
  };

  const handleOverlayClick = (e: Event) => {
    if ((e.target as HTMLElement).classList.contains('command-dialog-overlay')) {
      selectedCommand.value = null;
    }
  };

  return (
    <div class="command-dialog-overlay" onClick={handleOverlayClick}>
      <div class="command-dialog">
        <div class="dialog-header">
          <div class="dialog-title-row">
            <h2>{command.name}</h2>
            <div class={`status-indicator ${getStatusClass(status)}`}>
              <span class="status-dot" />
              {healthStatus && healthStatus !== 'Unknown' && (
                <span class={`health-dot ${healthStatus === 'Healthy' ? 'healthy' : 'unhealthy'}`} />
              )}
              <span>{getStatusText(status)}</span>
            </div>
          </div>
          <button class="dialog-close" onClick={() => selectedCommand.value = null}>&times;</button>
        </div>

        <div class="dialog-controls">
          {isOneTime ? (
            <button
              class="btn-secondary"
              onClick={handleToggle}
              disabled={isRunning}
            >
              {isRunning ? <span class="spinner" /> : 'Run'}
            </button>
          ) : (
            <button
              class={`toggle ${isRunning ? 'active' : ''}`}
              onClick={handleToggle}
              title={isRunning ? 'Stop' : 'Start'}
            />
          )}
          {!isOneTime && (
            <button
              class="btn-secondary"
              onClick={doRestart}
              disabled={isRestarting}
            >
              {isRestarting ? <span class="spinner" /> : 'Restart'}
            </button>
          )}
          <button class="btn-secondary" onClick={handleEdit}>Edit</button>
          <button class="btn-danger" onClick={handleDelete}>Delete</button>
        </div>

        {status.type === 'Error' && (
          <div class="dialog-error">
            {(status as { type: 'Error'; message: string }).message}
          </div>
        )}

        <div class="dialog-details">
          <div class="detail-row">
            <span class="detail-label">Command</span>
            <code class="detail-value">{command.command}</code>
          </div>
          <div class="detail-row">
            <span class="detail-label">Directory</span>
            <code class="detail-value">{command.cwd}</code>
          </div>
          {command.command_type === 'OneTime' && (
            <div class="detail-row">
              <span class="detail-label">Type</span>
              <span class="detail-value">One-Time</span>
            </div>
          )}
          {command.health_check_url && (
            <div class="detail-row">
              <span class="detail-label">Health URL</span>
              <code class="detail-value">{command.health_check_url}</code>
            </div>
          )}
          {Object.keys(command.env).length > 0 && (
            <div class="detail-row">
              <span class="detail-label">Env Vars</span>
              <div class="detail-value">
                {Object.entries(command.env).map(([k, v]) => (
                  <div key={k}><code>{k}={v}</code></div>
                ))}
              </div>
            </div>
          )}
          {command.tags && command.tags.length > 0 && (
            <div class="detail-row">
              <span class="detail-label">Tags</span>
              <div class="detail-value">
                {command.tags.map((tag, i) => (
                  <span class="tag-chip-display" key={i}>{tag}</span>
                ))}
              </div>
            </div>
          )}
        </div>

        <LogViewer commandId={id} fullSize />

        {confirmAction === 'delete' && (
          <ConfirmDialog
            title="Delete Command"
            message={`Are you sure you want to delete "${command.name}"?`}
            confirmLabel="Delete"
            danger
            onConfirm={doDelete}
            onCancel={() => setConfirmAction(null)}
          />
        )}
        {confirmAction === 'restart' && (
          /* TODO: Remove later */
          <ConfirmDialog
            title="Restart Command"
            message={`"${command.name}" is currently running. Restart it?`}
            confirmLabel="Restart"
            onConfirm={doRestart}
            onCancel={() => setConfirmAction(null)}
          />
        )}
      </div>
    </div>
  );
}
