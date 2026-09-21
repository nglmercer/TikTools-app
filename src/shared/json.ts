/**
 * Canonical JSON value types shared by every domain.
 *
 * These lived under `src/automation/contracts/generated/`; they are plain
 * structural types with no automation semantics, so generic layers
 * (`src/plugin-ui/`, `src/shared/`, plugin packages) import them from
 * here instead of reaching into automation. The generated mirror
 * re-exports these definitions (see `generatedJsonValueSource`).
 */

export type JsonPrimitive = null | boolean | number | string;
export type JsonValue = JsonPrimitive | JsonObject | JsonArray;
export type JsonObject = { [key: string]: JsonValue | undefined };
export type JsonArray = JsonValue[];
