import { useState, useEffect } from 'preact/hooks';
import { api } from '../lib/api';
import { commands, editingCommand, isFormOpen, type CommandType } from '../lib/store';

export function CommandForm() {
  const editing = editingCommand.value;
  const [name, setName] = useState('');
  const [cwd, setCwd] = useState('');
  const [command, setCommand] = useState('');
  const [commandType, setCommandType] = useState<CommandType>('Process');
  const [envRows, setEnvRows] = useState<{ key: string; value: string }[]>([]);
  const [healthCheckUrl, setHealthCheckUrl] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState('');
  const [autoStart, setAutoStart] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (editing) {
      setName(editing.name);
      setCwd(editing.cwd);
      setCommand(editing.command);
      setCommandType(editing.command_type || 'Process');
      const env = editing.env || {};
      const rows = Object.entries(env).map(([key, value]) => ({ key, value }));
      setEnvRows(rows.length > 0 ? rows : []);
      setHealthCheckUrl(editing.health_check_url || '');
      setTags(editing.tags || []);
      setAutoStart(editing.enabled);
    } else {
      setName('');
      setCwd('');
      setCommand('');
      setCommandType('Process');
      setEnvRows([]);
      setHealthCheckUrl('');
      setTags([]);
      setAutoStart(false);
    }
    setError('');
  }, [editing]);

  const handleClose = () => {
    isFormOpen.value = false;
    editingCommand.value = null;
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');

    if (!name.trim() || !cwd.trim() || !command.trim()) {
      setError('Name, working directory, and command are required');
      return;
    }

    // Build env map from rows
    const env: Record<string, string> = {};
    for (const row of envRows) {
      if (row.key.trim()) {
        env[row.key.trim()] = row.value;
      }
    }

    const hcUrl = healthCheckUrl.trim() || null;

    setSaving(true);
    try {
      if (editing) {
        await api.updateCommand(editing.id, name.trim(), cwd.trim(), command.trim(), autoStart, env, hcUrl, commandType, tags);
        commands.value = commands.value.map(c =>
          c.id === editing.id
            ? { ...c, name: name.trim(), cwd: cwd.trim(), command: command.trim(), enabled: autoStart, env, health_check_url: hcUrl, command_type: commandType, tags }
            : c
        );
      } else {
        const newCommand = await api.addCommand(name.trim(), cwd.trim(), command.trim(), env, hcUrl, commandType, tags);
        commands.value = [...commands.value, newCommand];
      }
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleOverlayClick = (e: Event) => {
    if ((e.target as HTMLElement).classList.contains('command-form-overlay')) {
      handleClose();
    }
  };

  const addEnvRow = () => {
    setEnvRows([...envRows, { key: '', value: '' }]);
  };

  const removeEnvRow = (index: number) => {
    setEnvRows(envRows.filter((_, i) => i !== index));
  };

  const updateEnvRow = (index: number, field: 'key' | 'value', val: string) => {
    setEnvRows(envRows.map((row, i) => i === index ? { ...row, [field]: val } : row));
  };

  return (
    <div class="command-form-overlay" onClick={handleOverlayClick}>
      <form class="command-form" onSubmit={handleSubmit}>
        <h2>{editing ? 'Edit Command' : 'Add Command'}</h2>

        {error && (
          <div style={{ color: 'var(--error)', fontSize: '14px', marginBottom: '16px' }}>
            {error}
          </div>
        )}

        <div class="form-group">
          <label>Type</label>
          <div class="command-type-selector">
            <button
              type="button"
              class={`type-option ${commandType === 'Process' ? 'active' : ''}`}
              onClick={() => setCommandType('Process')}
            >
              Process
            </button>
            <button
              type="button"
              class={`type-option ${commandType === 'OneTime' ? 'active' : ''}`}
              onClick={() => setCommandType('OneTime')}
            >
              One-Time
            </button>
          </div>
        </div>

        <div class="form-group">
          <label for="name">Name</label>
          <input
            id="name"
            type="text"
            value={name}
            onInput={(e) => setName((e.target as HTMLInputElement).value)}
            placeholder="e.g., Dev Server"
            autoFocus
          />
        </div>

        <div class="form-group">
          <label for="cwd">Working Directory</label>
          <input
            id="cwd"
            type="text"
            value={cwd}
            onInput={(e) => setCwd((e.target as HTMLInputElement).value)}
            placeholder="e.g., /Users/me/project"
          />
        </div>

        <div class="form-group">
          <label for="command">Command</label>
          <textarea
            id="command"
            value={command}
            onInput={(e) => setCommand((e.target as HTMLTextAreaElement).value)}
            placeholder="e.g., npm run dev"
          />
        </div>

        <div class="form-group">
          <label>
            Environment Variables
            <button type="button" class="btn-env-add" onClick={addEnvRow}>+ Add</button>
          </label>
          {envRows.map((row, i) => (
            <div class="env-row" key={i}>
              <input
                type="text"
                value={row.key}
                onInput={(e) => updateEnvRow(i, 'key', (e.target as HTMLInputElement).value)}
                placeholder="KEY"
                class="env-key"
              />
              <span class="env-eq">=</span>
              <input
                type="text"
                value={row.value}
                onInput={(e) => updateEnvRow(i, 'value', (e.target as HTMLInputElement).value)}
                placeholder="value"
                class="env-value"
              />
              <button type="button" class="icon-btn danger env-remove" onClick={() => removeEnvRow(i)}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          ))}
        </div>

        {commandType === 'Process' && (
          <div class="form-group">
            <label for="health-check-url">Health Check URL (optional)</label>
            <input
              id="health-check-url"
              type="text"
              value={healthCheckUrl}
              onInput={(e) => setHealthCheckUrl((e.target as HTMLInputElement).value)}
              placeholder="e.g., http://localhost:3000/health"
            />
          </div>
        )}

        <div class="form-group">
          <label>Tags</label>
          <div class="tag-input-container">
            {tags.map((tag, i) => (
              <span class="tag-chip" key={i}>
                {tag}
                <button type="button" class="tag-remove" onClick={() => setTags(tags.filter((_, idx) => idx !== i))}>
                  &times;
                </button>
              </span>
            ))}
            <input
              type="text"
              class="tag-input"
              value={tagInput}
              onInput={(e) => setTagInput((e.target as HTMLInputElement).value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  const val = tagInput.trim();
                  if (val && !tags.includes(val)) {
                    setTags([...tags, val]);
                  }
                  setTagInput('');
                }
              }}
              placeholder="Type and press Enter"
            />
          </div>
        </div>

        {commandType === 'Process' && editing && (
          <div class="form-group">
            <label class="auto-start-label">
              <button
                type="button"
                class={`toggle ${autoStart ? 'active' : ''}`}
                onClick={() => setAutoStart(!autoStart)}
              />
              Start when app launches
            </label>
          </div>
        )}

        <div class="form-actions">
          <button type="button" class="btn-secondary" onClick={handleClose}>
            Cancel
          </button>
          <button type="submit" class="btn-primary" disabled={saving}>
            {saving ? 'Saving...' : editing ? 'Save Changes' : 'Add Command'}
          </button>
        </div>
      </form>
    </div>
  );
}
