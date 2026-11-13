---
title: "Lançamentos, pacotes e controles do atualizador"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
translationRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
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

## Revise a confiança do atualizador {#review-updater-trust}

`tiktools-app-update` usa metadados de canal assinados e bytes de pacote assinados. Chave pública confiável, habilitação de feed e recursos publicados verificados são requisitos independentes. Somas SHA256 de pacotes sozinhas não habilitam o atualizador do aplicativo. Mantenha chaves de atualização do aplicativo distintas das chaves de publicadores de plugins.

O host limita downloads e redirecionamentos, e o encaminhamento de instalação deve aguardar o encerramento seguro do núcleo/WebView. A instalação AppImage também exige `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` na compilação. NSIS precisa de seu próprio controle `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. Código existente e um teste unitário de assinatura não justificam habilitar nenhum dos dois controles nas compilações de lançamento.

## Resultado esperado e status {#expected-result-and-status}

Empacotamento portátil e ferramentas de instaladores de plataforma estão implementados. Verificações/downloads dependem de provisionamento; substituição automática e reinício continuam experimentais. Instalações Linux desconhecidas/extraídas recorrem à instalação manual. Iniciar um instalador NSIS não permite que o processo antigo em encerramento informe sucesso do instalador.

## Verifique e recupere {#verify-and-recover}

Execute `bun scripts/task.ts rust test -p tiktools-app-update --locked` para dados locais de teste de protocolo/assinatura. Aceitação real de atualização, falha, preservação de perfil e reinício precisa de instalações compatíveis descartáveis. Mantenha o artefato verificado anterior disponível; uma publicação interrompida de feed/arquivo auxiliar deve falhar de forma segura. Não reative o encaminhamento enquanto o encerramento de plugins permanecer incerto. O [fluxo de lançamento](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) e o código do atualizador definem os controles atuais; planos históricos de lançamento não são uma lista de implantação.
