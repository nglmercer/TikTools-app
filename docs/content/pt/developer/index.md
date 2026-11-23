---
title: "Desenvolva o TikTools"
docId: "developer.overview"
translationKey: "developer.overview"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "cfa06c11d3c238ac914b0a3a6c2a65881a85b3ad8e6f5a14b7074c4973d4d8c8"
translationRevision: "57bf17344ddb816e0d14554f3a42fdead96c1cb36963958b83eeb92355345763"
order: 0
---

# Desenvolva o TikTools {#develop-tiktools}

O TikTools mantém um único AppCore nativo por trás de operações de controle tipadas. Vue renderiza as interfaces de desktop, navegador local e controlador; Winit/Wry é responsável pelas janelas de desktop. A CLI e o SDK usam as mesmas operações. Comece pela [configuração de desenvolvimento](setup.md), depois escolha o menor ciclo que cubra sua alteração.

## Escolha um ponto de entrada {#choose-an-entry-point}

| Trabalho | Ponto de entrada | O que comprova |
| --- | --- | --- |
| Componentes e stores Vue | Vite, Vitest e o ControlClient injetado | Comportamento da interface; sem validação de janela nativa. |
| Integração de desktop | `bun run start` | Integração de WebView e IPC de desenvolvimento. |
| Serviços nativos | Testes Cargo específicos | Contratos do núcleo, persistência e comportamento de serviços. |
| Acesso por navegador sem interface gráfica no host | `bun run dev` | Um host nativo, adaptador local e Vite com pareamento. |
| Documentação | `bun scripts/task.ts docs dev` | Markdown, VitePress e o servidor demo para os exemplos incorporados. |
| Demonstração pública | Entrada de demonstração separada e transporte simulado | Cenários determinísticos fictícios, não os serviços externos. |

## Responsabilidade pelo código-fonte {#source-ownership}

- `crates/tiktools-core`: serviços, operações e SQLite. Não depende de uma WebView.
- `crates/tiktools-control-api`: registro, roteamento JSON-RPC, IPC e descoberta.
- `crates/tiktools-desktop`: Winit/Wry, ponte com a thread de interface, ciclo de vida e recursos empacotados.
- `src/web/app`: inicialização, roteador e estrutura da interface. `pages` compõe `widgets` e `features`; `entities` é responsável por stores/contratos de domínio; `shared` contém interface e infraestrutura reutilizáveis.
- `src/automation/contracts/generated`: contratos de eventos do frontend versionados e gerados a partir de Rust. Não redigite esses esquemas em uma demonstração ou guia.
- `packages`: os pacotes TypeScript existentes de SDK/interface para plugins. Implementações de plugins ficam no repositório externo de plugins.

## Limites de contribuição {#contribution-boundary}

Mantenha explícitos os pontos de entrada nativo, de navegador local, controlador remoto e demonstração. A ausência de um host nativo é um erro em uma entrada real; ela não deve selecionar silenciosamente uma simulação. Descarte listeners, assinaturas e temporizadores com o escopo responsável. Preserve os lockfiles Rust/Bun e as verificações obrigatórias de CI.

A instalação pelo usuário pertence ao [guia do usuário](../user/installation.md). Essas verificações de desenvolvimento, sozinhas, não comprovam conectividade real com TikTok, renderização no OBS, compatibilidade com WebView nativa nem colaboração entre dois computadores.
