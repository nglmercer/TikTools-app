---
title: "Autoridade e autenticação de colaboração"
docId: "developer.collaboration"
translationKey: "developer.collaboration"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "cb0c8fc55416a0ade232a0272051d244ed5558d4b1f3aec87d37a7500af59475"
translationRevision: "cb0c8fc55416a0ade232a0272051d244ed5558d4b1f3aec87d37a7500af59475"
order: 190
---

# Autoridade e autenticação de colaboração {#collaboration-authority-and-authentication}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Permita que um dispositivo aprovado inspecione ou execute uma operação de escopo restrito em um host existente. Use um perfil de proprietário de teste, uma compilação com suporte remoto e um transporte configurado. Login de conta, participação em equipe do Cloud, propriedade local e sessão delegada direta são autoridades distintas.

## Estabeleça a cadeia de confiança {#establish-the-trust-chain}

A administração `collaboration.*` do proprietário local é responsável por configuração, convites, solicitações pendentes, identidades, concessões, sessões e auditoria. Descubra esquemas antes de configurar um receptor ou criar credenciais:

```sh
tiktools api schema collaboration.configure
tiktools api schema collaboration.invites.create
tiktools api call collaboration.status '{}' --format json
```

Um convite inicia o pareamento; ele não concede, por si só, autoridade de negócio. A aprovação do proprietário cria a sessão/concessões adequadas. HTTPS direto exige certificados validados ou confiança nativa fixada explicitamente. Os limites de token de portador nativo e cookie de navegador diferem. O `collaboration.hello` inicial vincula protocolo, instância do host, identidade, sessão, revisão da concessão, capacidades e limites negociados.

## Autorize cada operação {#authorize-each-operation}

Um método remoto compatível deve passar por sua lista de permissões, capacidade, restrição de recursos e verificações da sessão atual. Desativar/excluir fluxos usa metadados duráveis de comparação e troca. Ajustes de pontos são aditivos e têm seus próprios limites de concessão. Leituras remotas de automações são resumos fechados; rascunhos desativados sob autoria do criador não podem adquirir autoridade executável do proprietário apenas por serem salvos.

Revisão/adoção pelo proprietário e aprovação verificada vinculam definições atuais, dependências e procedência. Um manifesto de prévia informa dados de inspeção, não um token de execução. Revalide concessões e vínculos de recursos nos limites de execução, em vez de tratar a identidade do transporte como permissão permanente.

## Resultado esperado e status {#expected-result-and-status}

A colaboração nativa direta implementa leituras com escopo, mutações duráveis selecionadas e recuperação limitada. Equipes/relay do Cloud também têm seus próprios adaptadores; o caminho documentado de leitura por relay do Cloud não deve ser apresentado como moderação completa ou execução de rascunhos. Efeitos delegados restantes, gravações de domínio e aceitação entre vários PCs continuam experimentais. Consulte [clientes remotos](remote-client.md) e [Cloud](cloud.md) para o caminho selecionado.

## Recupere e verifique {#recover-and-verify}

Em revogação ou mudança de identidade, pare assinaturas e descarte projeções privadas. Use `collaboration.subscribe`, `snapshot` e `resume` conforme os domínios negociados e a validade do cursor. Resolva o ID original do comando após uma confirmação incerta. Não repita uma exclusão com um ID novo nem restaure autoridade de um recibo antigo. O [código de autorização](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-collaboration/src) e a política do registro definem o limite; passar pelo TLS sozinho não é um teste de autorização.
