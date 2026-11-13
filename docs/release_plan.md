# Historical CI and release hardening plan / Plan histórico de CI y publicación

This legacy URL is retained. Use the topic guides below for current instructions.

Se conserva esta dirección antigua. Usa las guías temáticas siguientes para las instrucciones actuales.

**Status / Estado:** historical design, release record or dated evidence; not current acceptance / diseño, registro de versión o evidencia histórica; no acredita aceptación actual.

**Source review:** The old package-script names and bundled-plugin release steps are obsolete. Current task.ts, immutable draft/upload verification and signed channel staging define the workflow; a checked-in workflow is not a completed release.

**Revisión del código:** Los nombres antiguos de scripts y los pasos de publicación con plugins incluidos quedaron obsoletos. task.ts, la verificación de borradores inmutables y el staging firmado definen el flujo actual; el workflow no demuestra una versión publicada.

<span id="1-fix-native-plugin-ffi-zero-length-buffer-handling"></span>
<span id="1-fix-native-plugin-ffi-zero-length-buffer-handling-"></span>
<span id="10-add-dependabot"></span>
<span id="10-add-dependabot-"></span>
<span id="11-pin-github-actions-safely"></span>
<span id="11-pin-github-actions-safely-"></span>
<span id="12-add-ci-friendly-project-scripts"></span>
<span id="12-add-ci-friendly-project-scripts-"></span>
<span id="13-establish-version-consistency"></span>
<span id="13-establish-version-consistency-"></span>
<span id="14-decide-how-packagejson-version-is-handled"></span>
<span id="14-decide-how-packagejson-version-is-handled-"></span>
<span id="15-add-release-workflow"></span>
<span id="15-add-release-workflow-"></span>
<span id="16-package-frontend-assets-with-the-desktop-executable"></span>
<span id="16-package-frontend-assets-with-the-desktop-executable-"></span>
<span id="17-release-build-matrix"></span>
<span id="17-release-build-matrix-"></span>
<span id="18-generate-sha-256-checksums"></span>
<span id="18-generate-sha-256-checksums-"></span>
<span id="19-use-github-release-generation"></span>
<span id="19-use-github-release-generation-"></span>
<span id="2-standardize-bun-as-the-frontend-package-manager"></span>
<span id="2-standardize-bun-as-the-frontend-package-manager-"></span>
<span id="20-do-not-reuse-legacy-electronsquirrel-release-files"></span>
<span id="20-do-not-reuse-legacy-electronsquirrel-release-files-"></span>
<span id="21-add-release-packaging-script"></span>
<span id="21-add-release-packaging-script-"></span>
<span id="22-add-license-file-if-missing"></span>
<span id="22-add-license-file-if-missing-"></span>
<span id="23-audit-secrets-and-generated-files"></span>
<span id="23-audit-secrets-and-generated-files-"></span>
<span id="24-preserve-webview-security-behavior"></span>
<span id="24-preserve-webview-security-behavior-"></span>
<span id="25-preserve-plugin-installer-security"></span>
<span id="25-preserve-plugin-installer-security-"></span>
<span id="26-review-process-plugin-timeout-behavior"></span>
<span id="26-review-process-plugin-timeout-behavior-"></span>
<span id="27-add-readme-ci-documentation"></span>
<span id="27-add-readme-ci-documentation-"></span>
<span id="28-update-development-documentation"></span>
<span id="28-update-development-documentation-"></span>
<span id="29-add-contributingmd-if-missing"></span>
<span id="29-add-contributingmd-if-missing-"></span>
<span id="3-add-frontend-eslint"></span>
<span id="3-add-frontend-eslint-"></span>
<span id="30-github-branch-protection-documentation"></span>
<span id="30-github-branch-protection-documentation-"></span>
<span id="31-validate-lockfiles"></span>
<span id="31-validate-lockfiles-"></span>
<span id="33-inspect-generated-desktop-package"></span>
<span id="33-inspect-generated-desktop-package-"></span>
<span id="34-avoid-unrelated-refactors"></span>
<span id="34-avoid-unrelated-refactors-"></span>
<span id="35-commit-structure"></span>
<span id="35-commit-structure-"></span>
<span id="36-final-report-required"></span>
<span id="36-final-report-required-"></span>
<span id="4-preserve-typescript-strictness"></span>
<span id="4-preserve-typescript-strictness-"></span>
<span id="6-add-main-ci-workflow"></span>
<span id="6-add-main-ci-workflow-"></span>
<span id="7-add-desktop-cross-platform-compilation-checks"></span>
<span id="7-add-desktop-cross-platform-compilation-checks-"></span>
<span id="8-add-github-codeql"></span>
<span id="8-add-github-codeql-"></span>
<span id="9-add-dependency-review"></span>
<span id="9-add-dependency-review-"></span>
<span id="_1-fix-native-plugin-ffi-zero-length-buffer-handling"></span>
<span id="_10-add-dependabot"></span>
<span id="_11-pin-github-actions-safely"></span>
<span id="_12-add-ci-friendly-project-scripts"></span>
<span id="_13-establish-version-consistency"></span>
<span id="_14-decide-how-package-json-version-is-handled"></span>
<span id="_15-add-release-workflow"></span>
<span id="_16-package-frontend-assets-with-the-desktop-executable"></span>
<span id="_17-release-build-matrix"></span>
<span id="_18-generate-sha-256-checksums"></span>
<span id="_19-use-github-release-generation"></span>
<span id="_2-standardize-bun-as-the-frontend-package-manager"></span>
<span id="_20-do-not-reuse-legacy-electron-squirrel-release-files"></span>
<span id="_21-add-release-packaging-script"></span>
<span id="_22-add-license-file-if-missing"></span>
<span id="_23-audit-secrets-and-generated-files"></span>
<span id="_24-preserve-webview-security-behavior"></span>
<span id="_25-preserve-plugin-installer-security"></span>
<span id="_26-review-process-plugin-timeout-behavior"></span>
<span id="_27-add-readme-ci-documentation"></span>
<span id="_28-update-development-documentation"></span>
<span id="_29-add-contributing-md-if-missing"></span>
<span id="_3-add-frontend-eslint"></span>
<span id="_30-github-branch-protection-documentation"></span>
<span id="_31-validate-lockfiles"></span>
<span id="_33-inspect-generated-desktop-package"></span>
<span id="_34-avoid-unrelated-refactors"></span>
<span id="_35-commit-structure"></span>
<span id="_36-final-report-required"></span>
<span id="_4-preserve-typescript-strictness"></span>
<span id="_6-add-main-ci-workflow"></span>
<span id="_7-add-desktop-cross-platform-compilation-checks"></span>
<span id="_8-add-github-codeql"></span>
<span id="_9-add-dependency-review"></span>
<span id="bug-fixes"></span>
<span id="bug-fixes-"></span>
<span id="changed-files"></span>
<span id="changed-files-"></span>
<span id="ci"></span>
<span id="ci-"></span>
<span id="definition-of-done"></span>
<span id="definition-of-done-"></span>
<span id="frontend-job"></span>
<span id="frontend-job-"></span>
<span id="objective"></span>
<span id="objective-"></span>
<span id="package-json"></span>
<span id="packagejson"></span>
<span id="packagejson-"></span>
<span id="release"></span>
<span id="release-"></span>
<span id="required-changes"></span>
<span id="required-changes-"></span>
<span id="rust-job"></span>
<span id="rust-job-"></span>
<span id="security"></span>
<span id="security-"></span>
<span id="tiktools--ci-security-release--code-hardening-task"></span>
<span id="tiktools--ci-security-release--code-hardening-task-"></span>
<span id="tiktools-—-ci-security-release-code-hardening-task"></span>

**Release engineering / Ingeniería de versiones** (Developer / Desarrollo): [English](content/en/developer/releases.md) · [Español](content/es/developer/releases.md).

**Contributing / Contribuir** (Developer / Desarrollo): [English](content/en/developer/contributing.md) · [Español](content/es/developer/contributing.md).

<span id="32-required-local-validation"></span>
<span id="32-required-local-validation-"></span>
<span id="5-add-rust-formatting-and-lint-validation"></span>
<span id="5-add-rust-formatting-and-lint-validation-"></span>
<span id="_32-required-local-validation"></span>
<span id="_5-add-rust-formatting-and-lint-validation"></span>
<span id="add-tests"></span>
<span id="add-tests-"></span>
<span id="verification"></span>
<span id="verification-"></span>

**Testing / Pruebas** (Developer / Desarrollo): [English](content/en/developer/testing.md) · [Español](content/es/developer/testing.md).

**Exact historical source / Fuente histórica exacta:** [original Markdown](internal/archive/docs/release_plan.md).

Archived byte for byte from commit [`80c5cb1f`](https://github.com/nglmercer/TikTools-app/blob/80c5cb1fbfa45eaeb1c086c71320a15e565d8541/docs/release_plan.md); SHA-256 `38d352ff29779a3a064fa838b8d5fe34b6c83feb65077d30cd1c18185d12ba3c`.

Conservado byte por byte; los resultados fechados y las afirmaciones obsoletas se mantienen como antecedentes. Consulta [el mapa de migración](migration-map.json) para la auditoría y los destinos de cada sección.
