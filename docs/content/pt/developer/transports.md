---
title: "Transportes atuais e restrições de plataforma"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "293f8c00709f3796e5d6bab335df654aef5d9d379c19d622934bf1f8a95591c0"
translationRevision: "293f8c00709f3796e5d6bab335df654aef5d9d379c19d622934bf1f8a95591c0"
order: 200
---

# Transportes atuais e restrições de plataforma {#current-transports-and-platform-restrictions}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Selecione o transporte que realmente existe para a compilação e plataforma de destino. Leia [API de controle](control-api.md) e [colaboração](collaboration.md). Um transporte carrega operações serializadas; ele não deve introduzir um segundo núcleo nem conceder autoridade por si só.

## Escolha o limite {#choose-the-boundary}

| Caminho | Implementação atual | Limite importante |
| --- | --- | --- |
| WebView de desktop | IPC do Wry e cliente de controle compartilhado | O tempo de vida da GUI/thread pertence ao desktop |
| CLI local | Socket Unix ou pipe nomeado Windows; host stdio explícito | Mesmo diretório de dados e proprietário do aplicativo |
| Navegador local | Adaptador nativo de loopback mais `browser-transport.ts` | Host nativo pareado, não uma simulação ou servidor remoto público |
| Remoto direto | Gateway HTTPS/WSS autenticado e transportes SDK/navegador | TLS, hello, concessões e controles de recursos |
| Par nativo | `tiktools-peer`, fluxo iroh/QUIC criptografado | Identidade do par não é autoridade RPC |
| WebRTC nativo | `tiktools-rtc`, sinalização assinada e canais de dados | Verificações de negociação, concessão e rota continuam necessárias |

A interface de transporte TypeScript fica em `src/web/shared/api/control-transport.ts`. A configuração de pares nativos não habilita implicitamente relays públicos de terceiros ou descoberta. WebRTC é nativo Rust (`webrtc` 0.21.1), não uma dependência do `RTCPeerConnection` do navegador na interface de desktop.

## Inspecione a rota selecionada {#inspect-the-selected-route}

Descubra os métodos e capacidades do host conectado antes de parear ou exibir uma ação de controlador:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
bun scripts/task.ts test web remote-browser-transport
```

`tiktools-rtc/src/route.rs` observa o par de candidatos ICE selecionado para classificar conexão direta ou via relay. O estado pronto sozinho não comprova uma rota direta. A infraestrutura de sinalização/relay do Cloud é separada da autorização de negócio e deve preservar as verificações nativas de concessão.

## Resultado esperado e status {#expected-result-and-status}

IPC local e transporte de desktop são caminhos estabelecidos. Par nativo, WebRTC e integração do controlador têm código e testes específicos, mas a implantação e a aceitação entre vários PCs continuam experimentais. Textos legados que afirmam não existir implementação de pares/relay são históricos. O IPC local do Android funciona; perfis remotos persistentes não têm armazenamento de credenciais selecionado e addons/WebViews nativos de desktop continuam indisponíveis ali.

## Recuperação {#recovery}

Trate a desconexão como possível incerteza para operações aceitas. Reconectar deve invalidar projeções privadas, renegociar a identidade e retomar ou obter instantâneos de fluxos limitados. Nunca reduza verificações de certificado nem retorne silenciosamente a um proprietário local após falha remota. A [implementação RTC](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-rtc/src) e o grafo real de recursos substituem planos antigos de transporte.
