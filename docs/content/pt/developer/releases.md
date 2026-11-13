---
title: "Lançamentos, pacotes e controles do atualizador"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a0c1a45f5279fec0b2fae29f40f47090d0600237815b293e9600e28221ec969b"
translationRevision: "73f85def4b74b35d2a1665bd2e664b50b2cc66c85e5b810a7b642c1b373ddb7f"
order: 220
---

# Lançamentos, pacotes e controles do atualizador {#releases-packages-and-updater-gates}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Prepare um artefato de lançamento revisável e diferencie downloads verificados de instalação automática. Este é um fluxo de mantenedor que exige as ferramentas de plataforma adequadas e material de assinatura protegido. Contribuidores comuns podem inspecionar empacotamento e verificações de versão sem publicar nem criar uma tag de lançamento.

## Prepare artefatos consistentes {#prepare-consistent-artifacts}

`package.json`, `Cargo.toml` do workspace e `Cargo.lock` compartilham a versão do aplicativo. Use o executor de tarefas em vez de editar uma versão isoladamente:

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

Uma mudança intencional de versão por um mantenedor usa `bun scripts/task.ts version bump v4.2.3` com a próxima versão selecionada. A compilação completa também cria recursos de widgets. O empacotamento de lançamento coloca o executável nativo ao lado de `web/index.html` e publica somas de verificação. Fluxos de instalador também compilam NSIS, DEB e AppImage e verificam seus recursos contra a preparação portátil. Pacotes de plugins continuam como instalações separadas de runtime.

Os cartões da página pública de versões usam uma captura dos metadados de lançamento, com a data em que os dados foram verificados. Para atualizá-la explicitamente pela API do GitHub, execute `bun scripts/refresh-docs-releases.ts` e revise a alteração antes de publicar a documentação. Essa atualização é separada da compilação: as compilações usam a captura local e podem funcionar sem acesso à rede. Os links de download representam apenas os arquivos que realmente foram enviados à versão; uma tag de código-fonte sozinha não cria um instalador nem um download disponível.

## Revise a confiança do atualizador {#review-updater-trust}

`tiktools-app-update` usa metadados de canal assinados e bytes de pacote assinados. Chave pública confiável, habilitação de feed e recursos publicados verificados são requisitos independentes. Somas SHA256 de pacotes sozinhas não habilitam o atualizador do aplicativo. Mantenha chaves de atualização do aplicativo distintas das chaves de publicadores de plugins.

O host limita downloads e redirecionamentos, e o encaminhamento de instalação deve aguardar o encerramento seguro do núcleo/WebView. A instalação AppImage também exige `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` na compilação. NSIS precisa de seu próprio controle `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. Código existente e um teste unitário de assinatura não justificam habilitar nenhum dos dois controles nas compilações de lançamento.

## Resultado esperado e status {#expected-result-and-status}

Empacotamento portátil e ferramentas de instaladores de plataforma estão implementados. Verificações/downloads dependem de provisionamento; substituição automática e reinício continuam experimentais. Instalações Linux desconhecidas/extraídas recorrem à instalação manual. Iniciar um instalador NSIS não permite que o processo antigo em encerramento informe sucesso do instalador.

## Verifique e recupere {#verify-and-recover}

Execute `bun scripts/task.ts rust test -p tiktools-app-update --locked` para dados locais de teste de protocolo/assinatura. Aceitação real de atualização, falha, preservação de perfil e reinício precisa de instalações compatíveis descartáveis. Mantenha o artefato verificado anterior disponível; uma publicação interrompida de feed/arquivo auxiliar deve falhar de forma segura. Não reative o encaminhamento enquanto o encerramento de plugins permanecer incerto. O [fluxo de lançamento](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) e o código do atualizador definem os controles atuais; planos históricos de lançamento não são uma lista de implantação.

## Detalhes de operação {#operational-details}

Empacotamento portable local exige versões coincidentes de fonte/manifestos/binário, widgets/web construídos e executável release. O exemplo usa a versão atual; altere tag só com mudança intencional. `package` valida binário/layout e grava `release/TikTools-<version>-<platform>.tar.gz` (Linux) ou `.zip` (Windows). `RELEASE_BINARY` seleciona outro binário. No Windows use atribuições PowerShell e `windows-x86_64`. Instaladores precisam das dependências e cargo-packager fixado do workflow, depois `bun scripts/package-installers.ts` com mesmo tag/plataforma. Portable não produz NSIS/DEB/AppImage.

Conjunção compilada: checks exigem `TIKTOOLS_APP_UPDATE_FEED_ENABLED=1` E `TIKTOOLS_APP_UPDATE_PUBLIC_KEY` válida; AppImage também `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` E origem suportada detectada; NSIS ainda `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. São valores de compilação. O workflow converte variável de repositório `TIKTOOLS_UPDATE_PUBLIC_KEY` em chave/feed. Sem ela pode publicar assets/checksums sem feed assinado. Com ela exige chave/senha protegidas, assinatura de manifesto exato/pacote, assets publicados verificados e promoção autenticada. Assinatura updater difere da assinatura OS e de editor de plugins.

Testes padrão omitem aceitação real AppImage. Dois ignorados exigem fixture real descartável informando 4.0.0 (asserção atual), FUSE e signer cargo-packager; substituição exige tela. Defina `TIKTOOLS_TEST_APPIMAGE`, liste testes e execute `preflight_runs_a_real_appimage` e `packager_replaces_a_running_disposable_appimage_without_touching_its_profile` com `-- --ignored --nocapture`. Árvore extraída não substitui montagem. Observe troca, versão, reinício e perfil. NSIS exige aceitação posterior independente; processo antigo não certifica sucesso.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
RELEASE_TAG=v4.2.2 RELEASE_PLATFORM=linux-x86_64 bun scripts/task.ts package
```

```powershell
$env:RELEASE_TAG = 'v4.2.2'
$env:RELEASE_PLATFORM = 'windows-x86_64'
bun scripts/task.ts package
```

```sh
TIKTOOLS_TEST_APPIMAGE=/absolute/path/disposable-v4.0.0.AppImage bun scripts/task.ts rust test -p tiktools-app-update --locked preflight_runs_a_real_appimage -- --ignored --nocapture
```
