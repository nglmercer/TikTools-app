#!/usr/bin/env bun
// Verifies that a Windows PE executable is a GUI-subsystem binary (subsystem 2).
// Replaces the former scripts/check-windows-pe.ps1 so CI and `task.ts build` need no PowerShell.
//
// Usage: bun scripts/check-windows-pe.ts <path-to-exe>
import { readFileSync, realpathSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const GUI_SUBSYSTEM = 2;
const IMAGE_FILE_HEADER_SIZE = 20;
const SUBSYSTEM_OFFSET = 68;

function fail(message: string): never {
  throw new Error(message);
}

export function checkWindowsPe(path: string): string {
  const resolved = realpathSync(path);
  const bytes = readFileSync(resolved);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (bytes.length < 0x40) fail(`PE validation failed: ${resolved} is too small`);

  const peOffset = view.getInt32(0x3c, true);
  if (peOffset < 0 || peOffset + 24 > bytes.length) fail('PE validation failed: invalid PE header offset');
  const signature = bytes.subarray(peOffset, peOffset + 4);
  if (signature[0] !== 0x50 || signature[1] !== 0x45 || signature[2] !== 0 || signature[3] !== 0) {
    fail(`PE validation failed: ${resolved} has no PE signature`);
  }

  const optionalHeader = peOffset + 4 + IMAGE_FILE_HEADER_SIZE;
  if (optionalHeader + 70 > bytes.length) fail('PE validation failed: optional header is truncated');
  const magic = view.getUint16(optionalHeader, true);
  if (magic !== 0x10b && magic !== 0x20b) {
    fail(`PE validation failed: unsupported optional header magic 0x${magic.toString(16).toUpperCase()}`);
  }

  // IMAGE_OPTIONAL_HEADER.Subsystem sits at the same offset for PE32 and PE32+.
  const subsystem = view.getUint16(optionalHeader + SUBSYSTEM_OFFSET, true);
  if (subsystem !== GUI_SUBSYSTEM) {
    fail(`Expected Windows GUI subsystem (${GUI_SUBSYSTEM}), found ${subsystem} in ${resolved}`);
  }
  return resolved;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const path = process.argv[2];
  if (!path) {
    console.error('usage: bun scripts/check-windows-pe.ts <path-to-exe>');
    process.exit(2);
  }
  try {
    console.log(`Windows GUI subsystem verified: ${checkWindowsPe(path)}`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
