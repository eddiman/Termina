import { signal, computed } from '@preact/signals';

export type CommandType = 'Process' | 'OneTime';

export interface Command {
  id: string;
  name: string;
  cwd: string;
  command: string;
  enabled: boolean;
  env: Record<string, string>;
  health_check_url: string | null;
  command_type: CommandType;
  tags: string[];
}

export type ProcessStatus =
  | { type: 'Stopped' }
  | { type: 'Running' }
  | { type: 'Exited'; code: number | null }
  | { type: 'Error'; message: string };

export type HealthStatus = 'Healthy' | 'Unhealthy' | 'Unknown';

// Preact Signals for reactive state
export const commands = signal<Command[]>([]);
export const statuses = signal<Record<string, ProcessStatus>>({});
export const healthStatuses = signal<Record<string, HealthStatus>>({});
export const isFormOpen = signal(false);
export const editingCommand = signal<Command | null>(null);
export const searchQuery = signal('');
export const selectedCommand = signal<string | null>(null);
export const selectedTags = signal<string[]>([]);

// Computed values
export const runningCount = computed(() =>
  Object.values(statuses.value).filter(s => s.type === 'Running').length
);

export const allTags = computed(() => {
  const tagSet = new Set<string>();
  for (const cmd of commands.value) {
    if (cmd.tags) {
      for (const tag of cmd.tags) {
        tagSet.add(tag);
      }
    }
  }
  return [...tagSet].sort();
});

export const sortedCommands = computed(() => {
  const query = searchQuery.value.toLowerCase().trim();
  const activeTags = selectedTags.value;
  let cmds = [...commands.value];
  if (query) {
    cmds = cmds.filter(
      c =>
        c.name.toLowerCase().includes(query) ||
        c.command.toLowerCase().includes(query) ||
        (c.tags && c.tags.some(t => t.toLowerCase().includes(query)))
    );
  }
  if (activeTags.length > 0) {
    cmds = cmds.filter(c =>
      c.tags && activeTags.every(tag => c.tags.includes(tag))
    );
  }
  return cmds.sort((a, b) => a.name.localeCompare(b.name));
});

// Helper to get status for a command
export function getStatus(id: string): ProcessStatus {
  return statuses.value[id] ?? { type: 'Stopped' };
}

// Helper to check if a status is an error
export function isErrorStatus(status: ProcessStatus): status is { type: 'Error'; message: string } {
  return status.type === 'Error';
}

// Helper to check if running
export function isRunningStatus(status: ProcessStatus): boolean {
  return status.type === 'Running';
}
