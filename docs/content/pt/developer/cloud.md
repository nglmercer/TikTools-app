---
title: "Contratos do Cloud, sincronização e revisão"
docId: "developer.cloud"
translationKey: "developer.cloud"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "06ae46545d1366d4f456c78a0e6583ea0338afbd496467e4d25b3abe1a5430fb"
translationRevision: "898da8f756958ffdd92bb80f4b2a5a9831c8453c5c6e746a964a5b5021435200"
order: 180
---

# Contratos do Cloud, sincronização e revisão {#cloud-contracts-synchronization-and-review}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Desenvolva recursos de configuração de conta e catálogo para a instância Cloud provisionada. A operação somente local continua válida. Para trabalho autenticado, use uma conta/backend dedicado de teste e o encaminhamento nativo da sessão; esta documentação não comprova aceitação de tokens de portador em produção nem entre dois dispositivos.

O responsável verificado pelo contrato é [nglmercer/RemakeBackend](https://github.com/nglmercer/RemakeBackend), com [contratos de configuração v2](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V2.md) e [superfícies v3](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V3.md) adicionais. Tipos portáteis e transporte de cliente sob responsabilidade do backend são representados localmente em `crates/cloud-contracts` e `crates/cloud-client`. Durabilidade e autenticação no desktop continuam sendo responsabilidades do host.

## Condicione e prepare operações {#gate-and-stage-operations}

1. Descubra métodos nativos com `rpc.discover`; um host mais antigo pode manter visualizações legadas.
2. Deixe o host verificar `/api/v2/capabilities` da instância antes de usar um endpoint condicionado a capacidades. Nunca fixe uma capacidade anunciada pelo servidor em uma flag apenas do navegador.
3. Use a origem provisionada pelo operador. Compilações de lançamento não aceitam substituições arbitrárias da origem em execução.
4. Visualize a migração e os conflitos, mantenha bases congeladas e gravações incertas e envie apenas a operação revisada.

```sh
tiktools api discover
tiktools api schema sync.v2.migrate
tiktools api schema globals.typed.replace
```

Esses comandos inspecionam esquemas; não migram uma conta nem salvam valores de variáveis.

## Implementação atual {#current-implementation}

V2 tem estado/armazenamento/coordenador nativos, migração v1 protegida, revisão de conflitos, reconstrução e reconciliação de plugins instalados. A adoção é explícita e limitada à conta; instalações legadas sem marcação permanecem no caminho legado. A aceitação completa entre dois desktops e alguns adaptadores de produto continuam incompletos. A configuração de época de gravação para toda a conta tem consequências irreversíveis no servidor explicadas na revisão da migração.

V3 adiciona catálogo, equipes, integrações, recursos, overlays e pacotes por trás de capacidades. Metadados de catálogo não substituem a verificação de plugins assinados. A importação usa o diário do proprietário; confira enabled ao aplicar/importar perfil. A aplicação de pacotes registra etapas interrompidas em vez de reproduzi-las automaticamente. O acesso de leitura por relay do Cloud não deve ser confundido com autoridade de mutação da colaboração direta.

## Resultado esperado e recuperação {#expected-result-and-recovery}

Alterações de configuração mantêm limites de identidade, revisão e análise. Changes de Automations é apenas local; Variables revisadas usam métodos nativos tipados reais e não sincronizam valores brutos. Changes de Overlays não está mesclado. Em um conflito, compare registros base/locais/novos do Cloud; em uma confirmação desconhecida, resolva a intenção mantida. Um erro HTTP que permite nova tentativa nunca autoriza repetir uma mutação. [Colaboração](collaboration.md) trata separadamente da autoridade sobre o host em execução.

## Detalhes de operação {#operational-details}

`TIKTOOLS_CLOUD_ORIGIN` usa runtime → valor compilado → padrão em debug, mas compilado → padrão em release, que ignora overrides runtime. Vazio, `off`, `none` desativam apenas na camada aplicável. Aceita origem HTTPS sem credenciais/caminho/query/fragmento; HTTP só loopback explícito. Debug isolado: `TIKTOOLS_CLOUD_ORIGIN=http://127.0.0.1:8787 bun run start`; release: `TIKTOOLS_CLOUD_ORIGIN=https://cloud.example bun scripts/task.ts rust build -p tiktools-desktop --release --locked`. Identidade de outra origem não restaura: confira origem/conta antes de autenticar. Padrão compilado não prova disponibilidade.

Siga [migração e recuperação por modo](../user/cloud/changes.md#operational-details). Coordene dispositivos antes da política irreversível de época, preserve contagens/resultados e consulte estado após barreira ambígua, sem repeti-la para descobrir se ocorreu. A recuperação congela entradas e revisão aprovadas; revisão vencida precisa ser reconstruída antes de retomar, que não envia escrita Cloud.
