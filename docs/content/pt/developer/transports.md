---
title: "Transportes atuais e restrições de plataforma"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a5fcd19211bdd279a0696cde31c26cc7444a128e50c3ca752489b25acb7cd9a"
translationRevision: "19b77bfb15a322e68e2d62a2676095caff614488a78296902c86926f76d5378c"
order: 200
---

# Transportes atuais e restrições de plataforma {#current-transports-and-platform-restrictions}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Selecione o transporte que realmente existe para a compilação e plataforma de destino. Leia [API de controle](control-api.md) e [colaboração](collaboration.md). Um transporte carrega operações serializadas; ele não deve introduzir um segundo núcleo nem conceder autoridade por si só.

## Escolha o limite {#choose-the-boundary}

| Caminho | Implementação atual | Limite importante |
| --- | --- | --- |
| WebView desktop | IPC Wry e cliente de controle compartilhado | Ciclo de vida da GUI/threads pertence ao desktop |
| CLI local | Socket Unix ou pipe nomeado do Windows; host stdio explícito | Mesmo diretório e proprietário do aplicativo |
| Navegador local | Adaptador loopback nativo mais `browser-transport.ts` | Host nativo pareado, não um servidor remoto público |
| Gateway HTTPS/WSS autenticado | Transportes SDK/navegador com TLS, hello, concessões e feature gates | Comandos remotos exigem concessão explícita |
| Plugin de controle remoto instalável | `tiktools.remote-control` sobre relay WSS com `remote.controller.ticket` | Propriedade do plugin; nenhum código remoto é compilado no host |

A interface TypeScript fica em `src/web/shared/api/control-transport.ts`. O controle remoto é um plugin instalável, não parte do binário do host: todo comando passa por `HostCallPolicy` (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`). Esta compilação não tem transporte peer/RTC nativo nem vínculo de host do Cloud Teams.

## Inspecione a rota selecionada {#inspect-the-selected-route}

Descubra os métodos e capacidades suportados pelo host conectado antes de parear ou exibir uma ação de controle:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
```

O pareamento e as concessões pertencem ao plugin: o ticket do controlador é de curta duração e uso único, e `remote.grant.list` / `remote.grant.revoke` são a visão autoritativa. Uma conexão de relay prova apenas o transporte, nunca autoridade RPC.

## Resultado esperado e estado {#expected-result-and-status}

O IPC local e o transporte desktop são caminhos estabelecidos. O controle remoto funciona pelo plugin instalável e um relay autenticado; não é um segundo runtime de host. Os códigos de login são enviados aos dispositivos conectados com TTL de cinco minutos, com OAuth como alternativa quando nenhuma conta está configurada. O IPC local no Android funciona com as ferramentas instrumentadas `mobile` (`info` / `exec` / `tty`); addons nativos e WebViews de desktop continuam indisponíveis lá.

## Recuperação {#recovery}

Trate a desconexão como possível incerteza para operações aceitas. A reconexão deve invalidar projeções privadas, renegociar a identidade e retomar ou capturar fluxos limitados. Nunca enfraqueça a verificação de certificados nem volte silenciosamente a um proprietário local após falha remota. Tickets expirados e concessões revogadas devem falhar fechados.

## Detalhes operacionais {#operational-details}

Mantenha as concessões com privilégio mínimo e prazo limitado; verifique-as após cada reconexão. Diagnostique separadamente identidade, expiração do ticket, escopo da concessão, acessibilidade do relay e tamanho do quadro. Quadros grandes falham na validação de enquadramento, não geram RPCs parciais. Teste sessões alteradas/revogadas com fixtures antes da aceitação real com dois dispositivos.
