---
title: "Encontre preferências e controles de teclado"
docId: "user.preferences.index"
translationKey: "user.preferences.index"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6790ab7171a90bda1df882c0f88626a5875da68bf4f624968b0e54815842030f"
translationRevision: "f109df7c60a73f42085174df738e19d5775ed761fd83df23a62b942b67ab744d"
order: 10
---

# Encontre preferências e controles de teclado {#find-preferences-and-keyboard-controls}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Defina o idioma do aplicativo, encontre a área de configuração correta e use confortavelmente os controles de teclado disponíveis. Você precisa da janela de desktop aberta; acesso à rede só é necessário para o serviço configurado.

## Encontre a configuração relevante {#find-the-relevant-setting}

1. Abra **Settings → General** para escolher o idioma da interface. As opções atuais são inglês (`en`), espanhol (`es`), português (`pt`), francês (`fr`), chinês simplificado (`zh`) e japonês (`ja`). A seleção de idioma não altera identificadores de eventos do TikTok nem nomes da API de plugins.
2. Revise os controles do atualizador em General: canal, verificações automáticas, notificações e verificação manual quando compatível. Verificações automáticas são opcionais. A disponibilidade de atualizações assinadas depende da configuração de assinatura publicada da compilação.
3. Use **Account** para identidade do Cloud e seus diagnósticos. Use **Collaboration** para configurar o acesso do proprietário e **Teams** quando o host expuser essa capacidade.
4. Abra **Plugins** para preferências gerais de plugins e os painéis Settings/Connection de cada plugin. As credenciais de serviço de um plugin não pertencem às configurações de General.
5. Abra **Automations → Variables** para valores compartilhados de execução, **Points** para regras de ganho e **Overlays** para design de widgets. Essas áreas de configuração ficam intencionalmente junto dos recursos que afetam.

A instalação automática é experimental e está desativada pelos controles de compilação do fluxo de lançamento atual. Uma preferência visível de verificação automática não promete instalação automática. Use o [processo de atualização](../updates.md) documentado e faça um [backup](backups.md) antes de mudar de versão.

## Use os controles de teclado disponíveis {#use-supported-keyboard-controls}

Use Tab e Shift+Tab para mover o foco entre controles comuns e Enter/Espaço para ativar botões focados. No inspetor de design de overlays, Ctrl/Meta+Z desfaz e Ctrl/Meta+Shift+Z ou Ctrl/Meta+Y refaz. Campos de texto mantêm seu próprio comportamento de desfazer. Esses são atalhos do editor, não atalhos globais de automação.

Escape fecha a gaveta de navegação estreita quando outro controle não o consumiu. Diálogos e editores podem tratar Escape por conta própria, incluindo um aviso de alterações não salvas. Automação por atalhos globais exige um plugin instalado compatível e as capacidades de backend informadas por ele.

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

Essas configurações e interações locais por teclado estão disponíveis; não se afirma aceitação completa de acessibilidade/plataforma. Prefira botões rotulados, tamanhos de janela legíveis e rolagem do feed pausada quando o movimento distrair. Se uma configuração parecer ausente, confira a página responsável e a capacidade do host antes de redefinir dados. Consulte [solução de problemas](troubleshooting.md) para controles indisponíveis e [perguntas frequentes](faq.md) para distinções comuns.

<DemoFrame scenario="settings" locale="pt" title="Demonstração: configurações" />
