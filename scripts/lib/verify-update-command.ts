/** Resolve the update-manifest verifier command.
 *
 * Releases normally use Cargo so the checked-out source is verified. Local
 * test loops can point TIKTOOLS_VERIFY_UPDATE_BIN at a prebuilt binary and
 * avoid paying Cargo startup on every invocation.
 */
export function verifyUpdateCommand(environment: Record<string, string | undefined> = process.env): string[] {
  const binary = environment.TIKTOOLS_VERIFY_UPDATE_BIN?.trim();
  return binary ? [binary] : ['cargo', 'run', '--quiet', '--locked', '-p', 'tiktools-app-update', '--bin', 'verify-update', '--'];
}
