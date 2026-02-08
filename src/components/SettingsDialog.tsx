import { useEffect, useRef, useState } from 'preact/hooks';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { api } from '../lib/api';
import { statuses, runningCount } from '../lib/store';
import { ConfirmDialog } from './ConfirmDialog';

const PRESETS: { label: string; value: string }[] = [
  { label: 'Custom', value: '' },
  { label: 'nvm', value: 'export NVM_DIR="$HOME/.nvm"\n[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"' },
  { label: 'fnm', value: 'eval "$(fnm env)"' },
  { label: 'Homebrew PATH', value: 'eval "$(/opt/homebrew/bin/brew shellenv)"' },
  { label: 'Source ~/.zshrc', value: 'unset npm_config_prefix 2>/dev/null\n[ -f "$HOME/.zshrc" ] && . "$HOME/.zshrc" 2>/dev/null' },
  { label: 'Source ~/.bashrc', value: '[ -f "$HOME/.bashrc" ] && . "$HOME/.bashrc" 2>/dev/null' },
  { label: 'pyenv', value: 'eval "$(pyenv init -)"' },
  { label: 'rbenv', value: 'eval "$(rbenv init -)"' },
];

interface Props {
  onClose: () => void;
}

export function SettingsDialog({ onClose }: Props) {
  const [autoStartEnabled, setAutoStartEnabled] = useState(false);
  const [isKilling, setIsKilling] = useState(false);
  const [orphanResult, setOrphanResult] = useState<string | null>(null);
  const [portInput, setPortInput] = useState('');
  const [portResult, setPortResult] = useState<string | null>(null);
  const [shellPath, setShellPath] = useState('');
  const [initScript, setInitScript] = useState('');
  const [shellSaved, setShellSaved] = useState(false);
  const [showDiscardConfirm, setShowDiscardConfirm] = useState(false);

  // Track the last-saved values to detect unsaved changes
  const savedShellPath = useRef('');
  const savedInitScript = useRef('');

  const isDirty = shellPath !== savedShellPath.current || initScript !== savedInitScript.current;

  useEffect(() => {
    isEnabled().then(setAutoStartEnabled).catch(() => {});
    api.getShellSettings().then((s) => {
      const sp = s.shell_path ?? '';
      const is = s.init_script ?? '';
      setShellPath(sp);
      setInitScript(is);
      savedShellPath.current = sp;
      savedInitScript.current = is;
    }).catch(() => {});
  }, []);

  const tryClose = () => {
    if (isDirty) {
      setShowDiscardConfirm(true);
    } else {
      onClose();
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') tryClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [shellPath, initScript]);

  const handleOverlayClick = (e: Event) => {
    if ((e.target as HTMLElement).classList.contains('command-form-overlay')) {
      tryClose();
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
        <div style={{ borderTop: '2px solid var(--border)', marginTop: '10px', paddingTop: '10px' }}>
          <div class="settings-label" style={{ marginBottom: '8px' }}>
            <span class="settings-title">Shell</span>
            <span class="settings-desc">Shell used to run commands (e.g. /bin/zsh, /bin/bash)</span>
          </div>
          <input
            value={shellPath}
            onInput={(e) => { setShellPath((e.target as HTMLInputElement).value); setShellSaved(false); }}
            placeholder="$SHELL or /bin/zsh"
            style={{ width: '100%', marginBottom: '10px' }}
          />
          <div class="settings-label" style={{ marginBottom: '8px' }}>
            <span class="settings-title">Init Script</span>
            <span class="settings-desc">Script to run before each command. Use this to set up your PATH, source shell configs, or unset conflicting variables.</span>
          </div>
          <div style={{ marginBottom: '8px' }}>
            <select
              style={{
                fontFamily: 'var(--font-pixel)',
                fontSize: '1.125rem',
                padding: '6px 8px',
                border: '2px solid var(--border)',
                borderRadius: 'var(--radius)',
                backgroundColor: 'var(--bg-primary)',
                color: 'var(--text-primary)',
                cursor: 'pointer',
              }}
              onChange={(e) => {
                const val = (e.target as HTMLSelectElement).value;
                if (val) {
                  setInitScript(val);
                  setShellSaved(false);
                }
              }}
              value=""
            >
              <option value="" disabled>Load a preset...</option>
              {PRESETS.filter(p => p.value !== '').map((p) => (
                <option key={p.label} value={p.value}>{p.label}</option>
              ))}
            </select>
          </div>
          <textarea
            value={initScript}
            onInput={(e) => { setInitScript((e.target as HTMLTextAreaElement).value); setShellSaved(false); }}
            placeholder={'[ -f "$HOME/.zshrc" ] && . "$HOME/.zshrc" 2>/dev/null'}
            rows={3}
            style={{ width: '100%', resize: 'vertical' }}
          />
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '8px' }}>
            <button
              class="btn-primary"
              onClick={async () => {
                try {
                  await api.updateShellSettings(
                    shellPath || null,
                    initScript || null,
                  );
                  savedShellPath.current = shellPath;
                  savedInitScript.current = initScript;
                  setShellSaved(true);
                } catch (err) {
                  console.error('Failed to save shell settings:', err);
                }
              }}
            >
              Save
            </button>
            {shellSaved && (
              <span style={{ fontSize: '1.125rem', color: 'var(--text-secondary)' }}>Saved. Changes apply to newly started commands.</span>
            )}
          </div>
          <div style={{ marginTop: '8px' }}>
            <a
              href="https://github.com/eddiman/Termina/blob/main/SHELL_SETUP.md"
              target="_blank"
              rel="noopener noreferrer"
              style={{
                fontSize: '1.125rem',
                color: 'var(--gold-dim)',
                textDecoration: 'underline',
                cursor: 'pointer',
              }}
            >
              See SHELL_SETUP.md for more examples
            </a>
          </div>
        </div>
      </div>
      {showDiscardConfirm && (
        <ConfirmDialog
          title="Unsaved Changes"
          message="You have unsaved shell settings. Discard changes?"
          confirmLabel="Discard"
          danger
          onConfirm={onClose}
          onCancel={() => setShowDiscardConfirm(false)}
        />
      )}
    </div>
  );
}
