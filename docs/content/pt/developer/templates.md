---
title: "Modelos de regras, perfis e revisão do proprietário"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "34c35d378f38a78866661b97c19113f83b5b5f2846cefa2b5076eb20fe9bdc21"
translationRevision: "2c51b48d317c5b20234e3d19e26974a975d686b731a3319770712d11d41294bc"
order: 160
---

# Modelos de regras, perfis e revisão do proprietário {#rule-templates-profiles-and-owner-review}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Importe regras reutilizáveis preservando propriedade local e capacidade de recuperação. Use um host privado, os provedores de ações necessários instalados e um `.tiktemplate.json` ou `.tikprofile.json` válido. Leia [automações](automations.md) antes de ativar uma regra importada. Importar uma biblioteca é trabalho de configuração, não prova de que sua integração externa está acessível.

## Prepare um documento {#prepare-a-document}

Um modelo v1 tem `templateVersion`, um ID portátil, esquema de parâmetros, ações e um evento vinculado. Um perfil tem `profileVersion`, parâmetros compartilhados e itens de modelos incorporados. Os analisadores autoritativos ficam em `crates/tiktools-core/src/services/templates/` e `src/web/entities/rule-template/lib/`; dados de teste em `tests/fixtures/templates/` mantêm os dois lados alinhados.

Os parâmetros são mesclados dos padrões do esquema para perfil, item e valores explícitos de diálogo/CLI. Um trecho inteiro <code v-pre>{{ params.* }}</code> preserva o tipo de valor JSON; trechos embutidos se tornam texto. Substituições de eventos permanecem para o momento de execução. Variáveis globais de execução são resolvidas depois e não devem ser incorporadas a um documento compartilhável como segredos de conta.

## Visualize e importe {#preview-and-import}

```sh
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose
tiktools template import rules.tiktemplate.json --disabled --param giftName=Rose
tiktools template list
tiktools template profile-export --out backup.tikprofile.json
```

Esses exemplos pressupõem um arquivo preparado e um host correspondente em execução. Na interface, Rule templates e importação de perfil preparam uma candidata para revisão. O coordenador nativo `automation.library.*` é responsável por transições atômicas da biblioteca e recuperação do resultado original; seu esquema atual pode ser descoberto pela [API de controle](control-api.md).

## Resultado esperado e status {#expected-result-and-status}

Importações revisadas de biblioteca criam registros vinculados coerentes e mantêm um diário do proprietário. O catálogo Cloud salva documentos de template; importar perfis e aplicar templates exige revisar enabled, sem presumir desativação. Provedores de ações ausentes ou desativados causam um erro de requisitos em vez de substituir silenciosamente uma ação. Um pacote de perfil selecionado controla o contexto de apresentação correspondente; seleção de perfil não é autoridade remota.

## Recupere e verifique {#recover-and-verify}

Em conflito de revisão, preserve a candidata, recarregue o instantâneo da biblioteca e revise a diferença real. Resolva um diário nativo pendente antes de criar outra intenção de importação. Não transforme uma atualização com falha em uma nova criação implícita. Execute `bun scripts/task.ts test web rule-templates` e `bun scripts/task.ts test web library-journal` para cobertura específica do frontend. O [serviço nativo de modelos](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) e os esquemas RPC gerados substituem referências antigas a `src/web/views/behavior`.

## Detalhes de operação {#operational-details}

Salve o primeiro JSON como `docs-log.tiktemplate.json`; o segundo é um `docs-pack.tikprofile.json` completo de uma entrada. Ambos desativam ações/eventos explicitamente. Precedência: padrão do schema → perfil → entrada → valores CLI/diálogo. Um span completo preserva tipo; um embutido vira texto. Enabled ausente usa true: aplique `--disabled` e confira a comparação UI.

Limites atuais: 8 ações e 12 filtros/template, 32 templates/perfil; texto UI 64 KiB/template e 256 KiB/perfil; expansão profundidade 64, 65.536 nós, 256 KiB/registro e 8 MiB total. Reduza conteúdo em excesso, sem ignorar validação. Limites de biblioteca, fonte e expansão são separados. Siga [recuperação do diário](automations.md#operational-details) antes de novo import com resultado incerto.

```json
{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false,"filters":[],"cooldownMs":1000,"cooldownScope":"global","runMode":"all"}}
```

```json
{"profileVersion":1,"id":"docs-pack","name":"Docs pack","description":"","params":{"message":"profile"},"templates":[{"params":{"message":"entry"},"template":{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false}}}]}
```

```sh
tiktools template import docs-log.tiktemplate.json --dry-run
tiktools template import docs-log.tiktemplate.json --disabled
tiktools template profile-import docs-pack.tikprofile.json --disabled
```
