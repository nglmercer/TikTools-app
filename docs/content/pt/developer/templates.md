---
title: "Modelos de regras, perfis e revisão do proprietário"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
translationRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
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

Importações revisadas de biblioteca criam registros vinculados coerentes e mantêm um diário do proprietário. Importações de modelos/perfis do catálogo do Cloud permanecem desativadas até ativação intencional do proprietário. Provedores de ações ausentes ou desativados causam um erro de requisitos em vez de substituir silenciosamente uma ação. Um pacote de perfil selecionado controla o contexto de apresentação correspondente; seleção de perfil não é autoridade remota.

## Recupere e verifique {#recover-and-verify}

Em conflito de revisão, preserve a candidata, recarregue o instantâneo da biblioteca e revise a diferença real. Resolva um diário nativo pendente antes de criar outra intenção de importação. Não transforme uma atualização com falha em uma nova criação implícita. Execute `bun scripts/task.ts test web rule-templates` e `bun scripts/task.ts test web library-journal` para cobertura específica do frontend. O [serviço nativo de modelos](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) e os esquemas RPC gerados substituem referências antigas a `src/web/views/behavior`.
