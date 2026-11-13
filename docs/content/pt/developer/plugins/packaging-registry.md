---
title: "Empacotamento, assinatura e registros de plugins"
docId: "developer.plugins.packaging-registry"
translationKey: "developer.plugins.packaging-registry"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b5c1b8dc63aafc5216467a00317c6670ac6744235abfc724fe372f3a28427ee8"
translationRevision: "88986b5baa400db98038166edb0f3ade2dcdae241d1aa442986e3463bdde8baa"
order: 280
---

# Empacotamento, assinatura e registros de plugins {#plugin-packaging-signing-and-registries}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Distribua um pacote cujos bytes, autoridade do publicador e compatibilidade com o host possam ser verificados independentemente. Comece com um plugin validado e um destino compatível. A assinatura precisa de chaves privadas de autor mantidas fora do repositório; delegação da raiz do registro é responsabilidade separada do operador.

## Prepare um arquivo revisável {#prepare-a-reviewable-archive}

```sh
tiktools-plugin validate --publish
tiktools-plugin build --release
tiktools-plugin keygen --id acme-1 --publisher acme --out /private/path/keys
```

`build` compila o pacote e cria arquivos `.plugin`. Entrada do manifesto, recursos, idiomas, saída da interface e dependências relevantes do runtime são incluídos com `checksums.json`. Compilações nativas/de processo precisam de um artefato de destino compatível; pacotes apenas JavaScript continuam portáteis até que addons nativos ou funcionalidades da plataforma os restrinjam.

Para um arquivo já compilado, use caminhos exatos em vez de um curinga não revisado:

```sh
tiktools-plugin sign dist-plugin/acme.hello-0.1.0.plugin --key /private/path/keys/acme-1.key.json
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json
```

Substitua o nome do arquivo pela saída real específica do destino da compilação. A assinatura cobre as somas de verificação, que cobrem os arquivos do pacote. Chaves públicas podem ser compartilhadas; arquivos de chaves privadas não devem entrar no controle de versão nem em registros.

## Política de confiança e atualização {#trust-and-update-policy}

O protocolo de registro estático tem autoridade assinada pela raiz e chaves delegadas de índice/publicador. Aprovação do namespace do publicador não é o mesmo que integridade de pacote, e um resumo do catálogo do Cloud não pode substituir nenhuma das cadeias de assinatura. O resolvedor verifica canal, mecanismo, esquema/protocolo/ABI, runtime e destino. Versões retiradas, revogadas ou incompatíveis continuam inelegíveis mesmo quando o hash do arquivo corresponde.

A instalação no host apresenta primeiro uma revisão. Atualizações automáticas exigem política permitida, pacote sem versão fixada, publicador assinado compatível e nenhuma nova autoridade; permissões ampliadas exigem revisão. A instalação pelo catálogo Cloud do produto é distinta da autoria de registro estático de baixo nível, e fluxos comuns do produto rejeitam URLs remotas arbitrárias de pacotes.

## Resultado esperado, recuperação e verificação {#expected-result-recovery-and-verification}

O instalador pode verificar bytes antes de aplicar um pacote revisado e manter estado de reversão/recuperação. A preparação de desenvolvimento fica fora do gerenciamento do instalador. Em falha de confiança, corrija delegação ou metadados do publicador em vez de usar flags para ignorar assinaturas como política normal de distribuição. Em inicialização incerta do runtime, pare a substituição e reconcilie o resultado mantido.

Use `bun scripts/task.ts rust test -p tiktools-plugin-package --locked` e os testes de registro para dados de teste offline. O [verificador de pacotes](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-package/src) e o [modelo de registro](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-plugin-registry/src/model.rs) definem os formatos técnicos; a infraestrutura de publicação precisa de aceitação separada pelo operador.

## Detalhes de operação {#operational-details}

Interprete o JSON da verificação, não apenas o código de saída. `verify` pode terminar com sucesso e `signature.status` igual a `unsigned` ou `untrusted`. Aceite somente `verified` com ID do pacote, editor e chave esperados e confira a política de distribuição. Uma chave local `--trust` comprova essa assinatura, não delegação do registro ou aprovação do namespace. Checksums isolados não comprovam autoridade nem confiança do updater. A automação abaixo deve falhar diante de resultado inesperado.

```sh
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json > verification.json
bun -e 'const r = await Bun.file("verification.json").json(); if (r.signature?.status !== "verified" || r.id !== "acme.hello" || r.signature.publisher !== "acme" || r.signature.keyId !== "acme-1") throw new Error("Package trust did not match policy");'
```
