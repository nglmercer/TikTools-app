---
title: "Configure sons e fala"
docId: "user.plugins.audio"
translationKey: "user.plugins.audio"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b6be887705d13dfe120f3e07dc4006ec0efa1472b6dbe0ccfc1ebfc1e83f3672"
translationRevision: "b196270cac210dccdc681480f77202d894cc91e5d9875af7c5d3a6bea36fe20e"
order: 20
---

# Configure sons e fala {#set-up-sounds-and-speech}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Reproduza um som previsível antes de conectar áudio a uma LIVE movimentada. Você precisa de um dispositivo de saída funcional e de um arquivo de áudio existente e compatível. A ação local **Play a sound** (`audio.play`) é integrada; provedores de fala, como uma integração SonicBoom instalada, exigem sua própria configuração de plugin/serviço.

## Teste um som local {#test-a-local-sound}

1. Em **Automations → Actions**, crie **Play a sound** (`audio.play`). Selecione um arquivo existente com o seletor de mídia.
2. Defina o volume como `0.2` para a primeira verificação. O intervalo aceito é de `0` a `1`; zero é silêncio.
3. Escolha a política de sobreposição: `allow` permite reprodução simultânea, `restart` reinicia o som e `drop` ignora uma nova instância enquanto ele já toca.
4. Salve a ação e leia a configuração antes de selecionar **Test**. Esse teste pode reproduzir som audível.
5. Ouça pelo dispositivo desejado. Após a reprodução local funcionar, vincule a ação a um evento com [filtro e intervalo de espera](../automations/recipes.md) conservadores.

A ação mantém uma referência de mídia baseada em caminho. O TikTools não copia o arquivo de áudio para o banco de dados. Mantenha o arquivo nesse caminho e inclua-o separadamente ao mover ou fazer backup da configuração.

## Configure fala opcional {#configure-optional-speech}

Instale o plugin de fala confiável por [Plugins](index.md). Nos controles de conexão/configurações, verifique endereço do serviço, autenticação e provedor, idioma e voz/perfil selecionados. Carregue um catálogo de vozes antes de testar uma frase curta não sensível. Uma voz ausente deve ser investigada como incompatibilidade de catálogo, propriedade ou provedor, em vez de substituída por um identificador inventado.

No SonicBoom, síntese, admissão de reprodução e conclusão audível são resultados diferentes. Uma resposta enfileirada ou iniciada não comprova que uma pessoa ouviu a frase. A fala clonada pode ser mais lenta que a chegada de mensagens de chat, portanto inspecione os diagnósticos de fila/saturação e mantenha limitada a admissão de eventos.

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

A reprodução de áudio local está disponível quando o host anuncia suporte a áudio. A fala continua dependente do plugin instalado e do serviço acessível; a aceitação por provedor/dispositivo/plataforma não é garantida aqui. Se não houver som, verifique existência do arquivo, volume, dispositivo de saída, estado de pausa e erro da execução. Se a fala falhar, confira o serviço e o catálogo de vozes antes de tentar novamente. Evite testes repetidos de efeitos durante uma transmissão e use [diagnósticos](../preferences/troubleshooting.md) sem compartilhar tokens ou texto privado falado.
