import { useComputed } from '@preact/signals';
import { api } from '../lib/api';
import { isErrorStatus, statuses, healthStatuses, selectedCommand, type Command, type ProcessStatus } from '../lib/store';

interface Props {
  command: Command;
}

export function CommandCard({ command }: Props) {
  const statusSignal = useComputed(() => statuses.value[command.id] ?? { type: 'Stopped' } as ProcessStatus);
  const status = statusSignal.value;
  const isRunning = status.type === 'Running';
  const hasError = isErrorStatus(status);
  const isOneTime = command.command_type === 'OneTime';

  const healthStatus = !isOneTime && command.health_check_url ? healthStatuses.value[command.id] : undefined;

  const handleToggle = async () => {
    try {
      if (isRunning) {
        await api.stopCommand(command.id);
        statuses.value = { ...statuses.value, [command.id]: { type: 'Stopped' } };
      } else {
        await api.startCommand(command.id);
        statuses.value = { ...statuses.value, [command.id]: { type: 'Running' } };
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      statuses.value = { ...statuses.value, [command.id]: { type: 'Error', message: errorMsg } };
    }
  };

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

  return (
    <div class="command-card" onClick={(e) => {
      if (!(e.target as HTMLElement).closest('button')) {
        selectedCommand.value = command.id;
      }
    }}>
      <div class="command-card-header">
        <div class="command-info">
          <div class="command-name">
            {command.name}
            {isOneTime && <span class="command-type-badge">One-Time</span>}
          </div>
          <div class="command-text">{command.command}</div>
          <div class="command-cwd">{command.cwd}</div>
          {command.tags && command.tags.length > 0 && (
            <div class="command-tags">
              {command.tags.map((tag, i) => (
                <span class="tag-chip-display" key={i}>{tag}</span>
              ))}
            </div>
          )}
        </div>
        <div class="command-actions">
          <div class={`status-indicator ${getStatusClass(status)}`}>
            <span class="status-dot" />
            {healthStatus && healthStatus !== 'Unknown' && (
              <span class={`health-dot ${healthStatus === 'Healthy' ? 'healthy' : 'unhealthy'}`} />
            )}
            <span>{getStatusText(status)}</span>
          </div>
          {isOneTime ? (
            <button
              class="btn-run"
              onClick={(e) => { e.stopPropagation(); handleToggle(); }}
              disabled={isRunning}
            >
              {isRunning ? <span class="spinner" /> : 'Run'}
            </button>
          ) : (
            <button
              class={`toggle ${isRunning ? 'active' : ''}`}
              onClick={(e) => { e.stopPropagation(); handleToggle(); }}
              title={isRunning ? 'Stop' : 'Start'}
            />
          )}
        </div>
      </div>
      {hasError && (
        <div style={{ color: 'var(--error)', fontSize: '12px', marginBottom: '12px' }}>
          {(status as { type: 'Error'; message: string }).message}
        </div>
      )}
    </div>
  );
}
