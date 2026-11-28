---
title: "Configurer les sons et la synthèse vocale"
docId: "user.plugins.audio"
translationKey: "user.plugins.audio"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "116cd7a6c7a48b62cb5ec92b41e8bc3869652ef552f920a92b97722c68779b9d"
translationRevision: "676b671e286ed930f18ad5ac7a9ecacb7d966bed728ff4b4b6504fbf53260382"
order: 20
---

# Configurer les sons et la synthèse vocale {#set-up-sounds-and-speech}

## Objectif et prérequis {#goal-and-prerequisites}

Jouer un son prévisible avant de relier l’audio à une activité LIVE intense. Il faut un périphérique de sortie fonctionnel et un fichier audio existant pris en charge. L’action locale **Play a sound** (`audio.play`) est intégrée ; les fournisseurs vocaux, comme une intégration SonicBoom installée, nécessitent leur propre configuration de plugin/service.

## Tester un son local {#test-a-local-sound}

1. Dans **Automations → Actions**, créez **Play a sound** (`audio.play`). Sélectionnez un fichier existant avec le sélecteur de médias.
2. Réglez le volume à `0.2` pour une première vérification. La plage prise en charge va de `0` à `1` ; zéro est silencieux.
3. Choisissez la politique de chevauchement : `allow` autorise les lectures simultanées, `restart` redémarre le son et `drop` ignore une nouvelle instance pendant sa lecture.
4. Enregistrez l’action et lisez sa configuration avant de sélectionner **Test**. Ce test peut émettre un son audible.
5. Écoutez avec le périphérique souhaité. Après une lecture locale réussie, reliez l’action à un événement avec un [filtre et un délai](../automations/recipes.md) prudents.

L’action conserve une référence de média fondée sur un chemin. TikTools ne copie pas le fichier audio dans sa base de données. Gardez le fichier à cet emplacement et incluez-le séparément lors du déplacement ou de la sauvegarde de votre configuration.

## Configurer la synthèse vocale facultative {#configure-optional-speech}

Installez le plugin vocal de confiance via [Plugins](index.md). Dans ses commandes de connexion/paramètres, vérifiez l’adresse du service, l’authentification et les choix de fournisseur, langue et voix/profil. Chargez un catalogue de voix avant de tester une courte phrase non sensible. Une voix manquante doit être examinée comme un problème de catalogue, de propriété ou de fournisseur, sans la remplacer par un identifiant inventé.

Pour SonicBoom, la synthèse, l’admission en lecture et la fin audible sont des résultats différents. Une réponse mise en file ou démarrée ne prouve pas qu’une personne a entendu la phrase. La voix clonée peut être plus lente que l’arrivée des messages : examinez les diagnostics de file/saturation et gardez l’admission des événements bornée.

## Résultat attendu, état et récupération {#expected-result-status-and-recovery}

La lecture audio locale est disponible lorsque l’hôte annonce la prise en charge audio. La synthèse vocale reste dépendante du plugin installé et d’un service accessible ; la validation du fournisseur, du périphérique ou de la plateforme n’est pas garantie ici. En cas de silence, vérifiez l’existence du fichier, le volume, la sortie, l’état de pause et l’erreur d’exécution. Si la synthèse échoue, vérifiez le service et le catalogue de voix avant de réessayer. Évitez les tests d’effets répétés pendant une diffusion et utilisez les [diagnostics](../preferences/troubleshooting.md) sans partager de jetons ou de texte privé prononcé.

## Opérations en ligne de commande {#command-line-operations}

Chaque opération de ce guide est aussi disponible depuis la CLI — hors ligne, scriptable et adaptée aux agents. Les lignes d'usage sont la syntaxe du parseur et restent en anglais dans toutes les langues.

<!--@include: ../../../../.generated/cli-domain-media.fr.md-->

Consultez la [référence complète des commandes](../../developer/cli.md) pour les conventions, la sortie JSON et les marqueurs d’opérations destructives.

<TourFrame id="automationActionMedia" locale="fr" title="Guide interactif : Actions overlay et son" />

<TourFrame id="automationActionTts" locale="fr" title="Guide interactif : Actions vocales (TTS)" />
