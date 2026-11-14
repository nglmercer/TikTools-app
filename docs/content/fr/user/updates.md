---
title: "Téléchargements et mises à jour"
docId: "user.start.updates"
translationKey: "user.start.updates"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6b92351b3ee8eb1b0745db65bbddb3eab21f4ae0a9945795c94147c9ca347e73"
translationRevision: "32717b10c0edf621049e1626bbe7a802dc9b31f0d81f58e884806f94c8bed5ca"
order: 40
---

# Téléchargements et mises à jour {#downloads-and-updates}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir le bon canal de publication et effectuer une mise à jour sans confondre sa recherche avec une installation automatique. Fermez les sessions actives et sauvegardez les données locales avant de changer de version, surtout lorsqu’une migration de base de données est documentée.

## Choisir un canal {#select-a-channel}

La [liste des versions](https://github.com/nglmercer/TikTools-app/releases) distingue les versions stables des préversions. Une version candidate telle que `-rc` est destinée aux vérifications. Le système de mise à jour distingue les flux stable et bêta ; il authentifie les métadonnées signées avant de proposer une version. Un fichier JSON non signé ou un installateur modifié ne constitue pas une mise à jour acceptée.

## Rechercher et installer {#check-and-install}

1. Ouvrez **Settings → General** et consultez l’état du système de mise à jour et la version installée.
2. Lancez la vérification disponible. Les compilations sans clé publique ou capacité de flux configurée indiquent que la vérification est indisponible.
3. Lisez les notes de version et choisissez le paquet de votre plateforme.
4. Lorsque la compilation propose un téléchargement ou un transfert manuel, suivez ce parcours. Le processus active les vérifications authentifiées seulement si la variable de clé publique est configurée ; l’installation automatique a des conditions supplémentaires.
5. Pour une mise à jour manuelle, fermez TikTools, remplacez ou installez le paquet complet vérifié, puis rouvrez-le. Conservez le répertoire de données séparément des fichiers de l’application.

## Résultat attendu {#expected-result}

Après redémarrage, l’application indique la version choisie et peut lire le profil local attendu. Les paquets portables ZIP/TAR nécessitent un remplacement manuel. Les dispositions inconnues ou extraites peuvent aussi imposer une procédure manuelle, même lorsqu’un installateur AppImage existe.

## Récupération et limites {#recovery-and-limitations}

Une vérification de signature échouée signifie que la mise à jour a échoué ; téléchargez une nouvelle copie officielle et vérifiez les sommes de contrôle de la version. Ne contournez pas la vérification. Le remplacement automatique AppImage et le transfert NSIS ont des conditions de compilation distinctes et sont expérimentaux. Il n’existe aucun retour arrière automatique général. Revenir à un ancien binaire n’annule pas la migration d’un schéma SQLite.

Signalez la version installée, le format du paquet et l’erreur de mise à jour issue de Diagnostics. Ne joignez pas de clés de signature, jetons d’invitation ou données de compte. Le [guide d’installation](installation.md) décrit les dispositions de paquet prises en charge.

## Détails de fonctionnement {#operational-details}

La capacité updater est fixée à la compilation. Les vérifications exigent feed et clé publique. AppImage automatique exige aussi la condition commune d’installation et une installation reconnue ; NSIS exige cette même condition ET sa condition spécifique. Les variables après installation n’activent pas ces options compilées. Portable/extrait/inconnu utilise le remplacement manuel. Vérifiez version, redémarrage et profil après installation ; téléchargement, checksum ou lancement ne prouvent pas son achèvement.
