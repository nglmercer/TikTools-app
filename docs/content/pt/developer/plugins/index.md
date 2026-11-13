---
title: "Visão geral do desenvolvimento de plugins"
docId: "developer.plugins.index"
translationKey: "developer.plugins.index"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
translationRevision: "fbb4afd7a336108b047205fadbd8e54fa6bb5dc04ac64dbf3216e26051c37c93"
order: 240
---

# Visão geral do desenvolvimento de plugins {#plugin-development-overview}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Escolha o menor limite de plugin que atenda à sua integração. O TikTools descobre pacotes de execução em raízes integradas, de usuário e de desenvolvimento; adicionar um pacote não exige recompilar um plugin pelo nome dentro do host. Use o devkit/SDK correspondente e um perfil privado de desenvolvimento antes de testar efeitos.

## Siga o caminho de autoria {#follow-the-authoring-path}

1. Comece pelo [guia inicial](quickstart.md) para gerar a estrutura, validar e compilar um pacote isolado.
2. Escolha um [ambiente de execução](runtimes.md) conforme dependências nativas e confiança, não apenas pela extensão do arquivo.
3. Declare [permissões e interface](permissions-ui.md) para que o host possa revisar efeitos e renderizar a configuração.
4. Use [empacotamento e registro](packaging-registry.md) para integridade, confiança no publicador e política de atualização.

A autoridade Rust de baixo nível é `crates/tiktools-plugin-api`: validação de manifestos, protocolo e contratos de interface. `crates/tiktools-plugin-sdk` oferece a superfície Rust de autoria; `packages/plugin-sdk` contém auxiliares TypeScript e o [esquema do editor](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json). O carregamento do ambiente de execução pertence a `crates/tiktools-plugin-loader`, não ao frontend.

## Entenda as contribuições {#understand-contributions}

Uma ação solicita um efeito por um resultado tipado. Fontes de eventos publicam acionadores; observadores recebem envelopes de domínio estáveis `{topic,data}` com entrega limitada. Processadores adicionam evidências sem efeitos em `event.intel`, com prazos, mesclagem determinística e isolamento de integridade. Configurações/ações declarativas permitem ao host renderizar configurações comuns. Uma página personalizada compilada usa um broker restrito de plugins em seu próprio quadro/janela; seu script nunca se torna parte do runtime Vue principal.

```sh
bun scripts/task.ts test bun packages
bun scripts/task.ts rust test -p tiktools-plugin-api --locked
```

Essas são verificações específicas do host/SDK, não validação de um pacote externo arbitrário.

## Resultado esperado e status {#expected-result-and-status}

Caminhos nativo, de processo, declarativo e `napi-vm` baseado em QuickJS estão implementados. WASM é uma opção de runtime sem mecanismo ativo de uso geral nas compilações normais. Integrações de exemplo ficam no [repositório plugins-example-tiktools](https://github.com/nglmercer/plugins-example-tiktools) separado; nomes de código em documentos antigos não as tornam recursos incluídos no produto.

## Recuperação {#recovery}

Para falha de descoberta, inspecione compatibilidade do manifesto, precedência das raízes, destino e caminho de entrada. Para uma intenção negada, inspecione capacidades declaradas e concedidas em vez de contornar o host. Uma inicialização incerta deve ser reconciliada antes da substituição. Limites nativos/de processo mantêm restrições de confiança do sistema operacional; o nome do runtime sozinho nunca estabelece uma sandbox.
