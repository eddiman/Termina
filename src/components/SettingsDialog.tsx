import { useEffect, useState } from 'preact/hooks';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { api } from '../lib/api';
import { statuses, runningCount } from '../lib/store';

interface Props {
  onClose: () => void;
}

export function SettingsDialog({ onClose }: Props) {
  const [autoStartEnabled, setAutoStartEnabled] = useState(false);
  const [isKilling, setIsKilling] = useState(false);
  const [orphanResult, setOrphanResult] = useState<string | null>(null);
  const [portInput, setPortInput] = useState('');
  const [portResult, setPortResult] = useState<string | null>(null);

  useEffect(() => {
    isEnabled().then(setAutoStartEnabled).catch(() => {});
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const handleOverlayClick = (e: Event) => {
    if ((e.target as HTMLElement).classList.contains('command-form-overlay')) {
      onClose();
    }
  };

  const toggleAutoStart = async () => {
    try {
      if (autoStartEnabled) {
        await disable();
        setAutoStartEnabled(false);
      } else {
        await enable();
        setAutoStartEnabled(true);
      }
    } catch (err) {
      console.error('Failed to toggle autostart:', err);
    }
  };

  const killAll = async () => {
    setIsKilling(true);
    try {
      const runningIds = Object.entries(statuses.value)
        .filter(([, s]) => s.type === 'Running')
        .map(([id]) => id);
      await Promise.all(runningIds.map(id => api.stopCommand(id)));
      const newStatuses = { ...statuses.value };
      for (const id of runningIds) {
        newStatuses[id] = { type: 'Stopped' };
      }
      statuses.value = newStatuses;
    } catch (err) {
      console.error('Failed to kill all:', err);
    } finally {
      setIsKilling(false);
    }
  };

  const running = runningCount.value;

  const killOrphans = async () => {
    try {
      const killed = await api.killOrphanedProcesses();
      setOrphanResult(killed > 0 ? `Killed ${killed} orphaned process group${killed > 1 ? 's' : ''}` : 'No orphaned processes found');
    } catch (err) {
      setOrphanResult(`Error: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  return (
    <div class="command-form-overlay" onClick={handleOverlayClick}>
      <div class="settings-dialog">
        <h2>Settings</h2>
        <div class="settings-row">
          <div class="settings-label">
            <span class="settings-title">Launch at Login</span>
            <span class="settings-desc">Start Termina automatically when you log in</span>
          </div>
          <button
            class={`toggle ${autoStartEnabled ? 'active' : ''}`}
            onClick={toggleAutoStart}
          />
        </div>
        <div class="settings-row">
          <div class="settings-label">
            <span class="settings-title">Kill All</span>
            <span class="settings-desc">Stop all running processes ({running})</span>
          </div>
          <button
            class="btn-danger"
            onClick={killAll}
            disabled={running === 0 || isKilling}
          >
            {isKilling ? <span class="spinner" /> : 'Kill All'}
          </button>
        </div>
        <div class="settings-row">
          <div class="settings-label">
            <span class="settings-title">Kill Orphans</span>
            <span class="settings-desc">Kill leftover processes from a previous session</span>
          </div>
          <button class="btn-danger" onClick={killOrphans}>
            Clean Up
          </button>
        </div>
        {orphanResult && (
          <div style={{ fontSize: '1.125rem', color: 'var(--text-secondary)', padding: '8px 0 0' }}>
            {orphanResult}
          </div>
        )}
        <div style={{ borderTop: '2px solid var(--border)', marginTop: '10px', paddingTop: '10px' }}>
          <div class="settings-label" style={{ marginBottom: '8px' }}>
            <span class="settings-title">Kill by Ports</span>
            <span class="settings-desc">Kill processes on specific ports (e.g. 3000-3005, 8080)</span>
          </div>
          <div style={{ display: 'flex', gap: '8px' }}>
            <input
              style={{ flex: 1 }}
              value={portInput}
              onInput={(e) => { setPortInput((e.target as HTMLInputElement).value); setPortResult(null); }}
              placeholder="3000-3005, 8080"
            />
            <button
              class="btn-danger"
              disabled={!portInput.trim()}
              onClick={async () => {
                setPortResult(null);
                try {
                  const killed = await api.killByPorts(portInput.trim());
                  setPortResult(killed > 0 ? `Killed ${killed} process${killed > 1 ? 'es' : ''}` : 'No processes found on those ports');
                } catch (err) {
                  setPortResult(`Error: ${err instanceof Error ? err.message : String(err)}`);
                }
              }}
            >
              Kill
            </button>
          </div>
          {portResult && (
            <div style={{ fontSize: '1.125rem', color: 'var(--text-secondary)', marginTop: '6px' }}>
              {portResult}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
