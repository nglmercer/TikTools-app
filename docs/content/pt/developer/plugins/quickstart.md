---
title: "Crie seu primeiro plugin"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "17f8e12522f04bad46b814f437e80c3f480b43331ed1278e49597878e2cee4f8"
translationRevision: "b4d02e1120be4045f61f9f2d14f19fbe7e4f188b67fcdcd02710c4aaf506c791"
order: 250
---

# Crie seu primeiro plugin {#build-a-first-plugin}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Produza um pacote validado que retorne um resumo de ação inofensivo. Use o devkit `tiktools-plugin` correspondente ao repositório e Bun 1.4.4 para autoria TypeScript. Crie esta amostra em um diretório de desenvolvimento separado; é um exemplo de autoria, não uma alteração de dependências do aplicativo.

## Gere a estrutura e valide {#scaffold-and-validate}

No checkout do TikTools, compile a CLI de autoria pelo executor de tarefas:

```sh
bun scripts/task.ts rust build -p tiktools-plugin-devkit --locked
```

Coloque `target/debug/tiktools-plugin` dessa compilação no caminho de ferramentas e, no diretório de autoria escolhido:

```sh
tiktools-plugin new acme.hello --template napi-vm-ts --dir acme-hello
cd acme-hello
tiktools-plugin validate
tiktools-plugin validate --publish
```

A estrutura gerada cria um manifesto schema-v3, ID estável com prefixo do publicador, descritor de ação, importação do SDK e configuração de compilação. Mantenha alinhados o runtime `napi-vm`, `apiVersion:1` e a entrada do módulo compilado. O nome é histórico; a execução atual usa QuickJS.

## Escreva e compile o código do plugin {#write-and-build-the-guest}

A ação gerada pode permanecer inteiramente no limite de resultado tipado:

```ts
import { actionConfig, actionResult, definePlugin } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    return actionResult(String(actionConfig(request).message ?? 'Olá'));
  },
});
```

Instale as dependências declaradas do pacote gerado nesse pacote separado e execute `tiktools-plugin build --release`. Ele compila JavaScript antecipadamente e empacota um arquivo. Não existe transpilador TypeScript em tempo de execução nem processo Node. O [SDK TypeScript](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/src/plugin.ts) define as assinaturas dos auxiliares; o esquema do manifesto auxilia o editor enquanto o analisador do host continua autoritativo.

## Resultado esperado e status {#expected-result-and-status}

`validate` informa problemas de compatibilidade e publicação; uma compilação bem-sucedida cria um pacote `.plugin` com manifesto, entrada e metadados de integridade. Uma ação apenas de resumo não precisa de efeitos adicionais do host. Se adicionar emissão de eventos, HTTP ou áudio, atualize as declarações e revise a autoridade solicitada antes de testar.

## Recuperação e verificação {#recovery-and-verification}

Se a entrada estiver ausente, compile antes de empacotar e confira seu caminho relativo ao manifesto. Não aponte o runtime de processo para JavaScript não compilado. Para preparação local, use `tiktools-plugin dev stage --into /absolute/path/dev-plugins` e um host privado de teste configurado para essa raiz. Preparação de desenvolvimento é separada da instalação; não comprova confiança em registro assinado. Continue com [limites de execução](runtimes.md) e [assinatura](packaging-registry.md) antes de distribuir a amostra.

## Detalhes de operação {#operational-details}

Leia `hostCompatibility`, `publishReady` e `warnings` de `validate`. Entrada não compilada é advertência; `--publish` falha por problemas de publicação, não por toda advertência de compatibilidade. Valide novamente após compilar, confira entrada/destino empacotados e teste o host de destino. Saída bem-sucedida antes da compilação não prova pacote utilizável.

Para Rust, crie `process-rust` ou `native-rust` em diretório separado. Ambos implementam `Plugin` do SDK e retornam `ActionResult::summary`; processo usa `tiktools_process_plugin!`, nativo usa `tiktools_export_native_plugin!` e `cdylib`. ABI/confiança nativa são específicas do alvo e em processo. Compile com a sintaxe de targets do devkit, mantenha o caminho gerado e use host privado. Prefira processo quando isolamento de falhas for adequado.

```sh
tiktools-plugin new acme.process --template process-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-process
tiktools-plugin new acme.native --template native-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-native
tiktools-plugin build acme-process --release
tiktools-plugin validate acme-process --publish
tiktools-plugin build acme-native --release
tiktools-plugin validate acme-native --publish
```
