---
title: "Plateformes et compilations natives Termux"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c7befe2be6f91bcf50ce4572aaa95d02723a315ba11ed1d94e6d499e802c2473"
translationRevision: "53c56ead4935beb9116e86f9794fcdf064bf92eb5624114d7e4afb6584845a56"
order: 20
---

# Plateformes et compilations natives Termux {#platforms-and-native-termux-builds}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir une compilation adaptée au système d’exploitation cible. Le développement de bureau nécessite Bun 1.4.4, Rust 1.95 ou ultérieur et les dépendances WebView de plateforme décrites dans la [configuration](setup.md). Termux compile directement la CLI Rust dans son répertoire privé Android ; le stockage partagé Android ne convient ni aux exécutables ni aux sockets Unix.

## Compiler l’hôte approprié {#build-the-appropriate-host}

Windows utilise MSVC Build Tools et WebView2. Strawberry Perl natif prend en charge OpenSSL embarqué ; ne le remplacez pas par MSYS Perl. Le bureau Linux nécessite GTK3, WebKitGTK 4.1, les bibliothèques de développement audio et `pkg-config`. L’intégration Winit/Wry appartient à `crates/tiktools-desktop/src/platform.rs` ; les services métier du cœur restent indépendants de la bibliothèque de fenêtres.

Sur un téléphone ARM64, vérifiez d’abord que `rustc -vV` indique `aarch64-linux-android`. Dans Termux :

```sh
pkg install git rust clang make cmake pkg-config python protobuf ca-certificates
export CARGO_BUILD_JOBS=2
bash scripts/termux/check.sh
cargo build -p tiktools-cli --locked
./target/debug/tiktools --standalone system info
```

Cargo est intentionnel ici : le parcours natif sur téléphone ne nécessite ni Bun ni Node. Sur un bureau capable d’exécuter Bun, passez Cargo par `bun scripts/task.ts rust ...` pour conserver la gestion de l’éditeur de liens et du cache. Avec deux terminaux sur téléphone, lancez `./target/debug/tiktools host --ipc` et utilisez le même `TIKTOOLS_HOME` pour les appels CLI suivants.

## Résultat attendu et état {#expected-result-and-status}

L’IPC local, SQLite et les plugins portables processus/QuickJS disposent de parcours Android implémentés. Les ressources de processus doivent cibler Android/Bionic. Les bibliothèques natives et extensions `.node` sont indisponibles sur Android. L’audio de bureau, les raccourcis globaux, notifications, sélecteurs de fichiers et WebViews n’ont pas de moteur téléphone. Les profils distants persistants échouent explicitement car Android n’a pas de coffre d’identifiants sélectionné ; les identifiants SDK peuvent rester en mémoire.

## Vérifier et récupérer {#verify-and-recover}

Exécutez `bash scripts/termux/doctor.sh`, `bash scripts/termux/test.sh` et `bash scripts/termux/smoke.sh` depuis la racine du dépôt. Signalez honnêtement les vérifications d’interface ignorées. Pour des symboles C++ `protoc` défaillants, comparez `env -u LD_LIBRARY_PATH protoc --version` avant de modifier les bibliothèques. Un ensemble de certificats CA manquant doit être réparé, jamais contourné. La compilation croisée seule ne démontre pas la validation sur téléphone. Le [script de vérification Termux](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/termux/smoke.sh) consigne les sondes ciblées et les restrictions restantes.

## Détails de fonctionnement {#operational-details}

Pour le navigateur local, construisez les assets avec index.html et la CLI avec `local-web`. Retirez les variables héritées et utilisez `TIKTOOLS_HOME` jetable ; démarrez d’abord l’hôte IPC. Dans un autre terminal avec la même racine, lancez l’adaptateur, ouvrez le lien loopback ou saisissez le code de six chiffres à usage unique (cinq minutes). Arrêtez séparément adaptateur et hôte. Loopback/pairing ne fournissent pas d’accès distant/LAN. Termux exige toolchain Android natif et assets web ; un binaire Linux desktop n’est pas Android.

```sh
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-cli --features local-web --locked
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools host --ipc
```

```sh
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools web --assets /absolute/path/to/TikTools/dist/web --port 4174
```
