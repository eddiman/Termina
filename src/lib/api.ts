import { invoke } from '@tauri-apps/api/core';
import type { Command, CommandType, ProcessStatus, HealthStatus } from './store';

export interface LogLine {
  text: string;
  stream: 'stdout' | 'stderr';
  timestamp: number;
}

export const api = {
  getCommands: () => invoke<Command[]>('get_commands'),

  startCommand: (id: string) => invoke<void>('start_command', { id }),

  stopCommand: (id: string) => invoke<void>('stop_command', { id }),

  restartCommand: (id: string) => invoke<void>('restart_command', { id }),

  getStatus: (id: string) => invoke<ProcessStatus>('get_status', { id }),

  getLogs: (id: string) => invoke<LogLine[]>('get_logs', { id }),

  getHealth: (id: string) => invoke<HealthStatus>('get_health', { id }),

  addCommand: (
    name: string,
    cwd: string,
    command: string,
    env?: Record<string, string>,
    healthCheckUrl?: string | null,
    commandType?: CommandType,
    tags?: string[],
  ) =>
    invoke<Command>('add_command', {
      name,
      cwd,
      command,
      env: env ?? {},
      healthCheckUrl: healthCheckUrl ?? null,
      commandType: commandType ?? 'Process',
      tags: tags ?? [],
    }),

  updateCommand: (
    id: string,
    name: string,
    cwd: string,
    command: string,
    enabled: boolean,
    env?: Record<string, string>,
    healthCheckUrl?: string | null,
    commandType?: CommandType,
    tags?: string[],
  ) =>
    invoke<void>('update_command', {
      id,
      name,
      cwd,
      command,
      enabled,
      env: env ?? {},
      healthCheckUrl: healthCheckUrl ?? null,
      commandType: commandType ?? 'Process',
      tags: tags ?? [],
    }),

  deleteCommand: (id: string) => invoke<void>('delete_command', { id }),

  getRunningCommands: () => invoke<string[]>('get_running_commands'),

  killOrphanedProcesses: () => invoke<number>('kill_orphaned_processes'),

  killByPorts: (ports: string) => invoke<number>('kill_by_ports', { ports }),

  quitApp: () => invoke<void>('quit_app'),
};
