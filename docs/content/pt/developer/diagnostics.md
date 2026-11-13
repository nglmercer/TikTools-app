---
title: "Diagnósticos e evidências seguras"
docId: "developer.diagnostics"
translationKey: "developer.diagnostics"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "bd492eda2007394d29d152e20ec0dba70614f06e6d29b75a3b741ed6b23f11b4"
translationRevision: "bd492eda2007394d29d152e20ec0dba70614f06e6d29b75a3b741ed6b23f11b4"
order: 130
---

# Diagnósticos e evidências seguras {#diagnostics-and-safe-evidence}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Colete evidências suficientes para localizar uma falha sem reter segredos de conta nem confundir diagnóstico com recuperação. Use um host privado de teste e saiba qual [transporte](transports.md) está selecionado. O aplicativo instala um coletor de sessão; componentes de status navegam até ele em vez de adicionar entradas durante a renderização.

## Inspecione uma falha {#inspect-a-failure}

1. Abra a ação de status contextual da conexão, plugin, rodada de sincronização ou atualizador com falha. O foco de teclado expõe a ação de registro do desktop assim como passar o ponteiro.
2. Filtre por origem, gravidade e texto e inspecione o contexto e a contagem de repetições da entrada selecionada.
3. Copie entradas selecionadas ou filtradas quando explicarem o problema. Revise exportações antes de compartilhá-las.
4. Compare a integridade nativa com a observação do frontend:

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system health
target/debug/tiktools --json system doctor
```

Uma sondagem de LIVE desconectada pode ser ignorada. Uma capacidade compilada não comprova disponibilidade na execução. O painel da interface é não modal no desktop; layouts estreitos usam `/debug` com um local de retorno. A exportação nativa de texto usa `system.saveFile`; navegadores baixam localmente.

## Entenda a retenção {#understand-retention}

`src/web/shared/diagnostics/log.ts` limita o histórico a 2.000 entradas e 2 MiB de UTF-8 serializado. Eventos consecutivos correspondentes de origem/entidade em até cinco segundos são combinados com uma contagem de repetições. A remoção de dados sensíveis ocorre antes da retenção, incluindo credenciais reconhecidas e locais secretos de URLs; segredos arbitrários de terceiros sem rótulo nem sempre podem ser reconhecidos. Não adicione configurações brutas, cookies, parâmetros RPC ou corpos HTTP aos coletores.

## Resultado esperado e status {#expected-result-and-status}

O visualizador oferece histórico de sessão limitado e navegação de contexto, não um registro durável de auditoria. Recarregar/desmontar limpa a memória. Limpar o histórico não limpa um erro de conexão nem corrige sua causa. IDs de requisição do Cloud podem ajudar a correlacionar um envelope seguro de falha; a cobertura de origens segue os coletores instalados e não garante a captura de todo erro de registro ou nativo.

## Verifique e recupere {#verify-and-recover}

Execute `bun scripts/task.ts test web diagnostics` e inspecione o [código do coletor](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/diagnostics). Reproduza remoção de dados sensíveis, descarte e comportamento de eventos repetidos com valores sintéticos. Para conflito, comando desconhecido ou credencial expirada, volte ao fluxo de recuperação responsável. Copiar uma entrada de diagnóstico coleta evidência; nunca repete uma ação nem aprova uma nova política de confiança.
