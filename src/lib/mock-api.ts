// Mock API for ui-only mode (VITE_UI_ONLY=true).
// Provides static dummy data so the UI can be developed without the Tauri backend.

import type { Command, ProcessStatus, HealthStatus } from './store';
import type { LogLine } from './api';

export const MOCK_COMMANDS: Command[] = [
  {
    id: 'mock-dummy',
    name: 'Dummy Card',
    cwd: '/tmp',
    command: 'echo "I am a dummy"',
    enabled: false,
    env: {},
    health_check_url: null,
    command_type: 'Process',
    tags: ['dummy', 'demo'],
  },
  {
    id: 'mock-process',
    name: 'Dev Server',
    cwd: '~/projects/my-app',
    command: 'npm run dev',
    enabled: true,
    env: { PORT: '3000', NODE_ENV: 'development' },
    health_check_url: 'http://localhost:3000',
    command_type: 'Process',
    tags: ['dev', 'demo'],
  },
  {
    id: 'mock-onetime',
    name: 'Build Project',
    cwd: '~/projects/my-app',
    command: 'npm run build',
    enabled: false,
    env: {},
    health_check_url: null,
    command_type: 'OneTime',
    tags: ['build', 'demo'],
  },
];

const mockStatuses: Record<string, ProcessStatus> = {
  'mock-dummy': { type: 'Stopped' },
  'mock-process': { type: 'Running' },
  'mock-onetime': { type: 'Stopped' },
};

const mockHealthStatuses: Record<string, HealthStatus> = {
  'mock-process': 'Healthy',
};

const mockLogs: Record<string, LogLine[]> = {
  'mock-dummy': [],
  'mock-process': [
    { text: '> my-app@1.0.0 dev', stream: 'stdout', timestamp: Date.now() - 5000 },
    { text: '> vite', stream: 'stdout', timestamp: Date.now() - 4800 },
    { text: 'VITE v5.0.0  ready in 312 ms', stream: 'stdout', timestamp: Date.now() - 4500 },
    { text: '  ➜  Local:   http://localhost:3000/', stream: 'stdout', timestamp: Date.now() - 4400 },
  ],
  'mock-onetime': [],
};

const noop = () => Promise.resolve(undefined as any);
const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));

export const mockApi = {
  getCommands: () => Promise.resolve([...MOCK_COMMANDS]),

  startCommand: async (id: string) => {
    await sleep(300);
    mockStatuses[id] = { type: 'Running' };
  },

  stopCommand: async (id: string) => {
    await sleep(200);
    mockStatuses[id] = { type: 'Stopped' };
  },

  restartCommand: async (id: string) => {
    await sleep(400);
    mockStatuses[id] = { type: 'Running' };
  },

  getStatus: (id: string): Promise<ProcessStatus> =>
    Promise.resolve(mockStatuses[id] ?? { type: 'Stopped' }),

  getLogs: (id: string): Promise<LogLine[]> =>
    Promise.resolve(mockLogs[id] ?? []),

  getHealth: (id: string): Promise<HealthStatus> =>
    Promise.resolve(mockHealthStatuses[id] ?? 'Unknown'),

  addCommand: async (
    name: string,
    cwd: string,
    command: string,
    env?: Record<string, string>,
    healthCheckUrl?: string | null,
    commandType?: Command['command_type'],
    tags?: string[],
  ): Promise<Command> => {
    const newCmd: Command = {
      id: `mock-${Date.now()}`,
      name,
      cwd,
      command,
      enabled: false,
      env: env ?? {},
      health_check_url: healthCheckUrl ?? null,
      command_type: commandType ?? 'Process',
      tags: tags ?? [],
    };
    MOCK_COMMANDS.push(newCmd);
    mockStatuses[newCmd.id] = { type: 'Stopped' };
    return newCmd;
  },

  updateCommand: async (
    id: string,
    name: string,
    cwd: string,
    command: string,
    enabled: boolean,
    env?: Record<string, string>,
    healthCheckUrl?: string | null,
    commandType?: Command['command_type'],
    tags?: string[],
  ) => {
    const idx = MOCK_COMMANDS.findIndex(c => c.id === id);
    if (idx !== -1) {
      MOCK_COMMANDS[idx] = {
        ...MOCK_COMMANDS[idx],
        name,
        cwd,
        command,
        enabled,
        env: env ?? {},
        health_check_url: healthCheckUrl ?? null,
        command_type: commandType ?? 'Process',
        tags: tags ?? [],
      };
    }
  },

  deleteCommand: async (id: string) => {
    const idx = MOCK_COMMANDS.findIndex(c => c.id === id);
    if (idx !== -1) MOCK_COMMANDS.splice(idx, 1);
    delete mockStatuses[id];
  },

  getRunningCommands: (): Promise<string[]> =>
    Promise.resolve(
      MOCK_COMMANDS
        .filter(c => mockStatuses[c.id]?.type === 'Running')
        .map(c => c.name),
    ),

  killOrphanedProcesses: () => Promise.resolve(0),

  killByPorts: (_ports: string) => Promise.resolve(0),

  quitApp: noop,

  getShellSettings: () =>
    Promise.resolve({ shell_path: null as string | null, init_script: null as string | null }),

  updateShellSettings: noop,

  openUrl: (_url: string) => Promise.resolve(),
};
