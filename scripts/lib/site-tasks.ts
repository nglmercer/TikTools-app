export interface SiteTaskRunner {
  run(...args: string[]): Promise<void>;
  script(file: string, ...args: string[]): Promise<void>;
  tool(name: string, ...args: string[]): Promise<void>;
  bun: string;
}

/** Explicit namespaces keep site dependencies and build directories separate from desktop. */
export async function runSiteTask(namespace: 'docs' | 'demo' | 'site', args: string[], runner: SiteTaskRunner): Promise<void> {
  const [action, ...extra] = args;
  if (extra.length) throw new Error(`${namespace} ${action ?? ''} takes no extra arguments.`);
  if (namespace === 'docs') {
    if (action === 'check') {
      await runner.run(runner.bun, 'docs/scripts/check.ts');
      await runner.run(runner.bun, 'docs/scripts/generate-references.ts', '--check');
      await runner.tool('vue-tsc', '--noEmit', '-p', 'docs/tsconfig.json');
    } else if (action === 'test') {
      await runner.run(runner.bun, 'test', './docs/scripts');
      await runner.tool('vitest', 'run', 'src/web/demo', 'src/web/features/guided-help');
      // The native-bridge absence regression serves the production frontend,
      // so build its independent output instead of relying on a stale dist/web.
      await runner.tool('vite', 'build');
      await runner.tool('playwright', 'test', '--config', 'playwright.site.config.ts');
    }
    else if (action === 'dev' || action === 'preview') await runner.run(runner.bun, 'run', '--cwd', 'docs', action);
    else if (action === 'build') {
      await runner.run(runner.bun, 'docs/scripts/check.ts');
      await runner.run(runner.bun, 'docs/scripts/generate-references.ts', '--check');
      await runner.run(runner.bun, 'run', '--cwd', 'docs', 'build');
    } else throw new Error('Expected docs dev|check|build|test|preview.');
    return;
  }
  if (namespace === 'demo') {
    if (action === 'dev') await runner.tool('vite', '--config', 'vite.demo.config.ts');
    else if (action === 'build') await runner.tool('vite', 'build', '--config', 'vite.demo.config.ts');
    else throw new Error('Expected demo dev|build.');
    return;
  }
  if (action === 'build') {
    await runSiteTask('docs', ['build'], runner);
    await runSiteTask('demo', ['build'], runner);
    await runner.script('assemble-site.ts');
    await runner.script('check-site.ts');
  } else if (action === 'preview') await runner.script('preview-site.ts');
  else throw new Error('Expected site build|preview.');
}
