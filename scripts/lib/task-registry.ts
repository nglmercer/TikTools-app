//! Task registry for `bun scripts/task.ts`.
//!
//! Built-in tasks register here instead of living in one hardcoded switch.
//! Plugin-owned tasks are contributed declaratively from `cliCommands` in each
//! installed plugin's `plugin.json`, so adding a plugin never means editing
//! `task.ts` again.

import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

export type TaskDef = {
  id: string;
  description: string;
  run: (args: string[]) => Promise<void>;
};

const tasks = new Map<string, TaskDef>();

export function registerTask(def: TaskDef): void {
  tasks.set(def.id, def);
}

export function getTask(id: string): TaskDef | undefined {
  return tasks.get(id);
}

export function listTasks(): TaskDef[] {
  return [...tasks.values()];
}

type ContributedCommand = {
  id: string;
  description?: string;
  actionType?: string;
};

function pluginRoots(): string[] {
  const roots: string[] = [];
  const explicit = process.env.TIKTOOLS_DEV_PLUGINS_DIR;
  if (explicit) roots.push(explicit);
  // Local staged development plugins in this checkout.
  roots.push(join(process.cwd(), '.dev-plugins'));
  const home = process.env.TIKTOOLS_HOME;
  if (home) roots.push(join(home, 'plugins'));
  return roots;
}

/// Discover `cliCommands` from installed plugin manifests. This is purely
/// local file reads: no host or credentials required, so `task.ts --help`
/// works even when TikTools is not running.
export function registerPluginTasks(): void {
  for (const root of pluginRoots()) {
    let entries: string[];
    try {
      entries = readdirSync(root);
    } catch {
      continue;
    }
    for (const entry of entries) {
      const manifestPath = join(root, entry, 'plugin.json');
      if (!existsSync(manifestPath)) continue;
      let manifest: { id?: string; cliCommands?: ContributedCommand[] };
      try {
        manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
      } catch {
        continue;
      }
      const pluginId = manifest.id;
      if (!pluginId || !Array.isArray(manifest.cliCommands)) continue;
      for (const command of manifest.cliCommands) {
        if (!command.id || typeof command.id !== 'string') continue;
        const id = `plugin:${pluginId}:${command.id}`;
        if (tasks.has(id)) continue;
        registerTask({
          id,
          description: command.description ?? `Run ${pluginId} ${command.id}`,
          // Delegate to the CLI's dynamic dispatcher; the host still owns
          // ownership checks and invocation via plugins.action.execute.
          run: async (args: string[]) => {
            const { createProcessRunner } = await import('./process-runner.ts');
            const runner = createProcessRunner({ cwd: process.cwd() });
            await runner.run(process.execPath, [
              'scripts/cargo-with-linker.mjs', 'run', '-p', 'tiktools',
              '--', '--standalone', 'plugin', pluginId, command.id, ...args,
            ]);
          },
        });
      }
    }
  }
}
