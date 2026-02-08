import { useEffect, useState } from 'preact/hooks';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { api } from './lib/api';
import { commands, statuses, healthStatuses, isFormOpen, editingCommand, runningCount, selectedCommand, type HealthStatus } from './lib/store';
import { CommandList } from './components/CommandList';
import { CommandForm } from './components/CommandForm';
import { CommandDialog } from './components/CommandDialog';
import { FilterBar } from './components/FilterBar';
import { ConfirmDialog } from './components/ConfirmDialog';
import { SettingsDialog } from './components/SettingsDialog';

export function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [quitConfirm, setQuitConfirm] = useState<string[] | null>(null);

  useEffect(() => {
    loadCommands();
    setupEventListeners();
    setupNotificationPermission();

    // Intercept smart/curly quotes from macOS and replace with straight quotes
    const handleBeforeInput = (e: InputEvent) => {
      if (!e.data) return;
      const cleaned = e.data
        .replace(/[\u00AB\u00BB\u201C\u201D\u201E\u201F\u2033\u2036]/g, '"')
        .replace(/[\u2018\u2019\u201A\u201B\u2032\u2035]/g, "'");
      if (cleaned !== e.data) {
        e.preventDefault();
        document.execCommand('insertText', false, cleaned);
      }
    };
    document.addEventListener('beforeinput', handleBeforeInput as EventListener);
    return () => document.removeEventListener('beforeinput', handleBeforeInput as EventListener);
  }, []);

  const setupNotificationPermission = async () => {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
  };

  const setupEventListeners = () => {
    // Listen for process exit events from the backend monitor
    listen<{ id: string; code: number | null; name: string }>('process-exited', (event) => {
      const { id, code } = event.payload;
      statuses.value = {
        ...statuses.value,
        [id]: { type: 'Exited', code },
      };
    });

    // Listen for notification requests from backend
    listen<string>('send-notification', async (event) => {
      const granted = await isPermissionGranted();
      if (granted) {
        sendNotification({ title: 'Termina', body: event.payload });
      }
    });

    // Listen for health status updates
    listen<{ statuses: Record<string, HealthStatus> }>('health-update', (event) => {
      healthStatuses.value = { ...healthStatuses.value, ...event.payload.statuses };
    });

    // Listen for open-command from tray menu
    listen<string>('open-command', (event) => {
      selectedCommand.value = event.payload;
    });

    // Listen for quit confirmation request
    listen('confirm-quit', async () => {
      try {
        const names = await api.getRunningCommands();
        setQuitConfirm(names);
      } catch {
        setQuitConfirm([]);
      }
    });
  };

  const loadCommands = async () => {
    try {
      const cmds = await api.getCommands();
      commands.value = cmds;

      const statusPromises = cmds.map(async (cmd) => {
        const status = await api.getStatus(cmd.id);
        return [cmd.id, status] as const;
      });

      const results = await Promise.all(statusPromises);
      const newStatuses: Record<string, typeof results[0][1]> = {};
      for (const [id, status] of results) {
        newStatuses[id] = status;
      }
      statuses.value = newStatuses;
    } catch (err) {
      console.error('Failed to load commands:', err);
    }
  };

  const handleAddClick = () => {
    editingCommand.value = null;
    isFormOpen.value = true;
  };

  return (
    <div class="app">
      <header class="app-header" onMouseDown={(e) => {
        if (!(e.target as HTMLElement).closest('button')) {
          getCurrentWindow().startDragging();
        }
      }}>
        <h1>Termina</h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          {runningCount.value > 0 && (
            <span style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
              {runningCount.value} running
            </span>
          )}
          <button
            class="icon-btn"
            onClick={() => setSettingsOpen(true)}
            title="Settings"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
          <button class="btn-primary" onClick={handleAddClick}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            Add
          </button>
        </div>
      </header>

      <FilterBar />

      <main class="app-content">
        <CommandList />
      </main>

      {selectedCommand.value && <CommandDialog />}
      {isFormOpen.value && <CommandForm />}
      {settingsOpen && <SettingsDialog onClose={() => setSettingsOpen(false)} />}
      {quitConfirm !== null && (
        <ConfirmDialog
          title="Quit Termina?"
          message={
            quitConfirm.length > 0
              ? `These commands are still running:\n${quitConfirm.map(n => `  - ${n}`).join('\n')}\n\nQuit anyway? All running commands will be stopped.`
              : 'Quit Termina?'
          }
          confirmLabel="Quit Anyway"
          danger
          onConfirm={async () => {
            setQuitConfirm(null);
            await api.quitApp();
          }}
          onCancel={() => setQuitConfirm(null)}
        />
      )}
    </div>
  );
}
