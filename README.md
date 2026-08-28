# Collomatique

Collomatique est un outil de construction automatique de **colloscopes** — les plannings de colles en classes préparatoires (CPGE). Il modélise le problème comme un **programme linéaire en nombres entiers** (ILP) et le résout avec un solveur (actuellement COIN-CBC).

## Avertissement

Collomatique est en développement actif et au stade alpha. L'interface et le format des fichiers peuvent changer sans préavis. Des bugs subsistent.

## Fonctionnalités

- **Colloscopes complexes** : différents types de périodicités, colles ponctuelles, gestion des TD, groupes différents par matière, règles complexes pour les créneaux du midi...
- **Import d'élèves depuis Pronote** (fichier CSV)
- **Export du colloscope en xlsx** : colloscope complet et fiches par groupe, avec couleurs et mise en page configurables

## Copies d'écran

Le colloscope : une ligne par créneau, une colonne par semaine. Il peut être modifié à la main ou construit automatiquement.

![Colloscope](screenshots/colloscope.png?raw=true "Le colloscope")

Le planning général : découpage de l'année en périodes et en semaines.

![Planning général](screenshots/general_planning.png?raw=true "Planning général")

Les matières, avec la taille des groupes et la périodicité de chacune.

![Matières](screenshots/subjects.png?raw=true "Édition des matières")

La résolution en cours.

![Résolution en cours](screenshots/solver_running.png?raw=true "Résolution du colloscope en cours")

**[Voir toutes les copies d'écran](screenshots/README.md)** — tous les panneaux de l'application.

## Installation

### Depuis un paquet binaire

Des paquets *devraient* être disponibles sur la [page des releases](https://github.com/christophcharles/Collomatique/releases). Ce n'est pas une promesse : il n'y en a pas forcément pour chaque version, ni pour chaque plateforme.

#### Flatpak (Linux)

Il faut `flatpak` installé et le dépôt flathub configuré : le paquet ne contient pas le runtime dont il a besoin (`org.gnome.Platform`), flatpak le récupère depuis flathub au moment de l'installation.

```bash
# Une seule fois, si flathub n'est pas déjà configuré
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo

flatpak install --user ./collomatique-<version>.flatpak
flatpak run fr.collomatique.Collomatique
```

#### Windows

Télécharger l'installeur (`.exe`) et le lancer, puis suivre les instructions.

### Depuis les sources

Avec [Nix](https://nixos.org/download/) (peut être installé sur n'importe quelle distribution Linux) :
```bash
nix-build
./result/bin/collomatique-gtk4

# Ou avec le flake
nix run
```

Sous Ubuntu, il faut d'abord installer Rust (dernière version recommandée) via [rustup](https://rustup.rs), puis :
```bash
sudo apt install build-essential libglib2.0-dev libpango1.0-dev libgdk-pixbuf-2.0-dev libgraphene-1.0-dev libgtk-4-dev libadwaita-1-dev coinor-libcbc-dev coinor-cbc libpython3-dev
cargo build --release
cargo rr --release
```
`cargo rr` est un alias défini dans `.cargo/config.toml` qui lance l'interface graphique GTK4 (`collomatique-gtk4`). Le mode `--release` est fortement recommandé : le solveur ILP est très lent en mode debug.

Le paquet `coinor-cbc` n'est nécessaire que pour l'exécution des tests.

Adwaita 1.7 est nécessaire. Collomatique ne compile pas sur Ubuntu 24.04 mais a été testé avec succès sur Ubuntu 25.10 et Ubuntu 26.04 (LTS au moment d'écrire).

## Résolution par programmation linéaire

Collomatique modélise la construction du colloscope comme un **programme linéaire en nombres entiers** (ILP — *Integer Linear Programming*).

Le principe :

- **Variables binaires** : chaque décision est représentée par une variable qui vaut 0 ou 1. Par exemple, « l'élève X passe en créneau Y » → 0 (non) ou 1 (oui).
- **Contraintes linéaires** : les règles du colloscope sont encodées sous forme d'inégalités. Par exemple, « chaque élève passe exactement une fois par période » se traduit par une somme de variables égale à 1.
- **Fonction objectif** : une expression linéaire à minimiser (ou maximiser) qui permet d'optimiser l'équilibrage, de respecter les préférences, etc.

**Exemple simplifié** — deux élèves (A, B), deux créneaux (1, 2), chacun doit passer exactement une fois :

```
Variables : xA1, xA2, xB1, xB2 ∈ {0, 1}

Contraintes :
  xA1 + xA2 = 1      (A passe exactement une fois)
  xB1 + xB2 = 1      (B passe exactement une fois)
  xA1 + xB1 ≤ 1      (au plus un élève par créneau 1)
  xA2 + xB2 ≤ 1      (au plus un élève par créneau 2)

Objectif : minimiser un coût d'équilibrage (par ex. écart entre créneaux)
```

Le solveur détermine alors que `xA1 = 1, xB2 = 1` (et les autres à 0) est une solution réalisable et optimale.

## Licence

Ce projet est distribué sous la licence **GNU Affero General Public License v3** (AGPL-3.0). Voir le fichier [LICENSE](LICENSE) pour le texte complet.

En résumé : toute version modifiée redistribuée ou exposée via un service réseau doit rester sous AGPL-3.0 et rendre son code source disponible.

## Organisation du code

Le projet est un workspace Rust rangé en trois groupes : `generic/` est la machinerie écrite pour être réutilisable d'un problème à l'autre, `colloscopes/` est l'application, `rooms/` un projet annexe inachevé.

### `generic/`

| Crate | Rôle |
|---|---|
| `ilp/` | Primitives ILP : expressions linéaires, contraintes, objectifs, variables, construction de problèmes, interface solveur |
| `ilp-modeler/` | Modéliseur ILP indépendant du problème : expansion paresseuse de variables, réification, objectification, composition par bundles |
| `ilp-modeler-derive/` | Macros dérivées pour `ilp-modeler` (`#[derive(DescribeVar)]`) |
| `collo-cbc/` | Interface C++ pour le solveur CBC, avec `CbcEventHandler` |
| `strategies/` | Stratégies de résolution au-dessus du solveur : résolution par étapes successives, recherche de la solution réalisable la plus proche d'une configuration donnée, perturbation aléatoire de celle-ci |
| `mps/` | Export de problèmes ILP au format MPS |
| `state/` | Traits et structures d'état d'un éditeur, indépendants de l'interface |
| `state-derive/` | Macros dérivées pour `state` : identifiants typés et références |
| `subprocesses/` | Exécution de la résolution dans un sous-processus, indépendante de l'interface |
| `rpc/` | Moitié générique du protocole RPC : transport, jobs ILP et stratégies |
| `rpc-engine/` | Moteur qui exécute ces jobs et rend compte de leur avancement |
| `time/` | Types pour représenter les jours, heures et durées dans Collomatique |

### `colloscopes/`

| Crate | Rôle |
|---|---|
| `gtk4/` | Interface graphique GTK4/Adwaita (Relm4) — c'est le binaire de l'application |
| `state-colloscopes/` | État du document colloscope et opérations élémentaires dessus |
| `ops/` | Opérations de haut niveau sur le document, telles qu'un utilisateur les fait (interface graphique et Python) |
| `storage/` | Lecture et écriture du fichier (JSON) ; le format est spécifié dans [`docs/file_format`](docs/file_format/file_format.md) |
| `constraints-colloscopes/` | Traduction des règles du colloscope en modèle ILP : périodicités, appariements, équilibrage, structure d'emploi du temps |
| `greedy-groups/` | Remplissage automatique des listes de groupes : un algorithme glouton, sans solveur |
| `settings/` | Ce qui relève de l'installation et de la personne, pas du document : version du build, réglages persistants |
| `ui-text/` | Les mots employés par l'application pour parler de ses propres données, partagés entre l'interface et l'API Python |
| `xlsx/` | Export du colloscope au format xlsx |
| `python/` | Le module Python `collomatique` (PyO3) |
| `python-runner/` | Cycle de vie de l'interpréteur et exécution des scripts |
| `rpc-colloscopes/` et `rpc-engine-colloscopes/` | Moitié colloscope du protocole et du moteur RPC : le document qui circule entre l'application et un script hébergé |
| `testgen-colloscopes/` | Génération déterministe de séquences d'opérations, pour les tests de propriétés |

### `rooms/`

Projet annexe, inachevé : la planification des salles. `rooms/` (outil en ligne de commande), `rooms-model/` (modèle de données) et `constraints-rooms/` (modélisation des contraintes).

### Hors des crates

`pkgs/` contient l'empaquetage (Nix, Flatpak, Windows), `docs/` la spécification du format de fichier, la conception de l'API Python et les todos, `examples/` des fichiers d'exemple, `scripts/` des scripts Python utilisables depuis l'application, et `screenshots/` les copies d'écran.
