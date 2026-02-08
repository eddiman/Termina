import { sortedCommands, isFormOpen } from '../lib/store';
import { CommandCard } from './CommandCard';

export function CommandList() {
  const cmds = sortedCommands.value;

  if (cmds.length === 0) {
    return (
      <div class="empty-state">
        <h2>No commands yet</h2>
        <p>Add your first command to get started</p>
        <button class="btn-primary" onClick={() => isFormOpen.value = true}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          Add Command
        </button>
      </div>
    );
  }

  return (
    <div class="command-list">
      {cmds.map(cmd => (
        <CommandCard key={cmd.id} command={cmd} />
      ))}
    </div>
  );
}
