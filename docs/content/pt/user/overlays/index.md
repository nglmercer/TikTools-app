---
title: "Prepare overlays para o OBS"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6062b2d3d3248eaa9ab1c8f73082c8d4a6bbe81d199f8a45b6c8c28dea617d46"
translationRevision: "796199f3b0a9f6c23f195bae46aadfb48620b906b0cd075dad47eaa2d958ff26"
order: 10
---

# Prepare overlays para o OBS {#prepare-overlays-for-obs}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Projete um widget legível e conecte sua fonte de navegador local ao OBS. Você precisa do TikTools em execução, da capacidade de gateway de overlays e do OBS no mesmo computador para o endereço de loopback padrão. A saída ao vivo também precisa do evento recebido relevante ou de uma fonte de ranking configurada.

## Projete e visualize {#design-and-preview}

1. Abra **Overlays** e leia o status do gateway. Escolha novos seguidores, presentes, chat, compartilhamentos, assinaturas, curtidas ou ranking.
2. Edite camadas, texto visível, cores, espaçamento e opções de avatar no inspetor de design. Use contraste suficiente e tamanho legível na resolução real da transmissão.
3. Reproduza a prévia para inspecionar o layout. O conteúdo da prévia ajuda a avaliar o design; não comprova a entrega de eventos da LIVE.
4. Use os controles Undo/Redo para edições locais e depois **Save**. Trocar o tipo de widget com alterações não salvas solicita que você as descarte.
5. No painel de prévia/fonte, copie a URL da fonte. A URL exibida mascara sua credencial; use o controle de cópia para obter a URL utilizável. Mantenha essa URL privada.
6. Adicione uma fonte de navegador no OBS e cole a URL copiada. Mantenha o TikTools em execução e valide um evento relevante antes de depender dele durante uma transmissão.

O gateway padrão é `127.0.0.1:17452`. Uma URL de loopback se refere ao computador que executa a fonte de navegador, portanto copiá-la para outro computador não alcança esse host automaticamente. Widgets de ranking também precisam de um ID de ranking selecionado; consulte o [status dos rankings](../points/rankings.md).

## Use a biblioteca do Cloud {#use-the-cloud-library}

Quando o host expuser capacidades de overlays do Cloud, abra a biblioteca em Overlays. Navegue por designs locais ou **My overlays**, use o editor para criar um overlay do Cloud ou abra um ID existente e revise-o. O editor do Cloud aceita texto, campos ao vivo, imagens e dimensões de tela; envios de imagens usam seu fluxo separado. Salvar o design local e criar/revisar no Cloud são operações distintas.

## Resultado esperado e status {#expected-result-and-status}

Designs locais, controles de gateway e fluxos do editor/biblioteca do Cloud condicionados às capacidades estão disponíveis. Um adaptador compartilhado **Changes** de overlays não está disponível nesta versão. OBS, LIVE real e cada sistema operacional ainda exigem aceitação no seu ambiente.

## Recuperação {#recovery}

Para saída ausente, verifique estado do gateway, recursos, credenciais, URL da fonte e tipo de evento selecionado. Para um conflito de revisão no Cloud, recarregue a revisão atual antes de editar novamente. Nunca publique o token da fonte em capturas de tela; informe o erro do gateway pelos [diagnósticos](../preferences/troubleshooting.md).

<DemoFrame scenario="overlays" locale="pt" tour="overlay" title="Demonstração: overlays" />

## Detalhes de operação {#operational-details}

Instale Event Gateway separadamente, habilite e confira execução antes das fontes OBS LIVE. Estados: missing → instalar; disabled → habilitar/conferir disponibilidade; stopped → iniciar; credentialUnavailable → reiniciar gateway uma vez; assetsMissing → reparar/reconstruir assets empacotados; starting → aguardar brevemente; unreachable → conferir porta/serviço loopback. Use credencial de widget da URL gerada. Ready ou prévia local não provam LIVE nem recepção OBS: confira com teste inofensivo na fonte pretendida.
