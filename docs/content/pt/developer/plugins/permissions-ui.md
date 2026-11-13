---
title: "Permissões de plugins e interface isolada"
docId: "developer.plugins.permissions-ui"
translationKey: "developer.plugins.permissions-ui"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
translationRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
order: 270
---

# Permissões de plugins e interface isolada {#plugin-permissions-and-isolated-ui}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Exponha configuração e efeitos intencionais do host sem importar scripts de plugins para o aplicativo principal. Use um manifesto validado, o pacote SDK/interface de plugins correspondente e um host privado de teste. Leia [confiança de execução](runtimes.md): uma capacidade de manifesto não é uma sandbox do sistema operacional para bibliotecas nativas ou executáveis de processo.

## Declare uma contribuição restrita {#declare-a-narrow-contribution}

Descreva campos de ações/configurações por esquemas JSON e dicas de interface do manifesto. Páginas declarativas simples renderizam componentes confiáveis do host. `events.publish` condiciona intenções de eventos; áudio do host também exige a capacidade `audio.play` e a permissão `audio.output`. Confira o [esquema canônico do manifesto](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json) e o analisador Rust em vez de copiar uma lista exaustiva de permissões para a documentação.

Processadores exigem sua contribuição de enriquecimento declarada e retornam anotações/visualizações/registros sem efeitos. Observadores de domínio exigem `events.subscribe` e recebem um envelope estável por entrega limitada, não o enum interno de eventos Rust nem identificadores arbitrários de transporte.

## Crie uma página personalizada isolada {#build-an-isolated-custom-page}

Um `ui` de manifesto com `apiVersion:1` e `mode:"webview"` aponta para recursos compilados em `ui/dist/`. O aplicativo principal hospeda um quadro de origem opaca com scripts habilitados; janelas destacadas opcionais usam um broker nativo restrito. Scripts personalizados nunca executam dentro do runtime Vue principal.

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
const stop = broker.subscribe('plugin.settings-changed', () => location.reload());
window.addEventListener('pagehide', stop, { once: true });
```

O host fornece a identidade do plugin. O cliente não pode escolher outro ID de plugin. Métodos do broker são limitados às próprias configurações/ações/opções/eventos mais idioma/tema. Uma página aberta sem broker falha explicitamente. Executar uma ação por uma página é um efeito real, mesmo que o botão diga teste.

## Resultado esperado e status {#expected-result-and-status}

Renderização declarativa, quadros incorporados e janelas nativas destacadas têm contratos e testes implementados. CORS/CSP, vínculo com a janela de origem e listas de permissões do broker nativo aplicam conjuntamente o limite; Shadow DOM não o faz. A propriedade IPC não configurável do WebKitGTK não precisa ser excluída porque o próprio manipulador nativo é restrito.

## Recupere e verifique {#recover-and-verify}

Para um quadro em branco, inspecione caminhos de recursos, CSP e CORS de origem opaca antes de flexibilizar a política. Para `broker unavailable`, verifique o contexto de hospedagem. Execute `bun scripts/task.ts test bun plugin-ui` e `bun scripts/task.ts test web plugin-webview-host`. O [pacote broker](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-ui/src/broker.ts) define o comportamento de envelope/versão e a semântica de desmontagem.
