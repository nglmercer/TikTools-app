---
title: "Contribua com alterações focadas"
docId: "developer.contributing"
translationKey: "developer.contributing"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "df2f1f1e0670dbfc40091e8bea0fa26c1821c4d7cb70f1d62ba79ae9f9e15072"
translationRevision: "df2f1f1e0670dbfc40091e8bea0fa26c1821c4d7cb70f1d62ba79ae9f9e15072"
order: 230
---

# Contribua com alterações focadas {#contributing-focused-changes}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Prepare uma alteração que outro contribuidor possa revisar e verificar. Leia o [CONTRIBUTING.md](https://github.com/nglmercer/TikTools-app/blob/remake/CONTRIBUTING.md) do repositório, os arquivos `AGENTS.md` aplicáveis e a [configuração](setup.md) antes de editar. Use Bun 1.4.4, Rust 1.95 ou mais recente quando necessário, lockfiles versionados e uma branch baseada no destino pretendido.

## Defina e implemente a alteração {#define-and-implement-the-change}

1. Inspecione `git status` e preserve trabalho de outras pessoas. Mantenha recursos gerados, credenciais e bancos de dados locais fora do patch.
2. Siga o responsável atual pelo código: núcleo Rust para comportamento de serviços, módulos da API de controle para contratos, cliente tipado para cobertura do SDK e o [mapa de camadas do frontend](frontend.md) para apresentação.
3. Para uma alteração de contrato, atualize sua definição canônica e regenere a saída derivada em conjunto. Não mantenha manualmente outra tabela completa de esquemas RPC ou eventos.
4. Mantenha implementações específicas de plugins em seus pacotes externos; código genérico do host deve descobrir manifestos em vez de importar um plugin pelo nome.
5. Explique o acionador concreto e o comportamento resultante, incluindo caminhos incompletos ou experimentais.

## Valide proporcionalmente {#validate-proportionately}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts contracts --check
```

Alterações de implementação nativa também precisam das verificações aplicáveis de formatação/lint/testes Rust e controles obrigatórios da CI. Uma alteração apenas de documentação deve usar verificações de documentação e revisar seus comandos/links de código, em vez de afirmar aceitação da execução nativa. O executor de tarefas do projeto define os pontos de entrada aceitos; use o repasse ao Cargo para flags específicas de alvos nativos.

## Resultado esperado e status {#expected-result-and-status}

A revisão contém alterações focadas de código, evidências significativas de validação e limites honestos. A política do repositório direciona pull requests para `remake`; tags de lançamento são controladas pelos mantenedores. As instruções de contribuição estão disponíveis, mas proteção de branches e configuração de verificações remotas são responsabilidades de administradores, não garantias de uma página Markdown.

## Recupere {#recover}

Se a validação falhar por dependência ou biblioteca de plataforma ausente, informe e corrija o ambiente em vez de enfraquecer uma verificação. Se arquivos gerados divergirem, regenere a partir de sua fonte responsável e inspecione a diferença. Nunca publique cookies de sessão, chaves, credenciais ou capturas de eventos não sanitizadas. Se código e um projeto legado divergirem, descreva a discrepância e use a implementação atual nas orientações públicas. Não sobrescreva arquivos pendentes de outro responsável para fazer uma limpeza ampla parecer consistente.
