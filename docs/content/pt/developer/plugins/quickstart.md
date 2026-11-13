---
title: "Crie seu primeiro plugin"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "61201a958231d15db3c9d5b2aad72fa33b1863224ebb395b7e7a879aa1eb16e8"
translationRevision: "61201a958231d15db3c9d5b2aad72fa33b1863224ebb395b7e7a879aa1eb16e8"
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
