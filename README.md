# Collomatique

Collomatique est un outil de construction automatique de **colloscopes** — les plannings de colles en classes préparatoires (CPGE). Il modélise le problème comme un **programme linéaire en nombres entiers** (ILP) et le résout avec un solveur (actuellement COIN-CBC).

## Avertissement

Collomatique est en développement actif et au stade alpha. L'interface et le format des fichiers peuvent changer sans préavis. Des bugs subsistent.

## Fonctionnalités

- **Périodicités complexes** : périodicité exacte, par bloc de semaines, nombre de colles dans l'année, blocs arbitraires
- **Construction automatique des groupes** par le solveur, avec possibilité d'exclure des élèves — attention, très lent pour l'instant
- **Import d'élèves depuis Pronote** (fichier CSV)
- **Gestion des incompatibilités de créneaux**
- **Export du colloscope en xlsx** : colloscope complet et fiches par groupe, avec couleurs et mise en page configurables

## À venir

- Complétion de colloscopes partiels
- Meilleurs réglages pour l'équilibrage
- Support d'autres solveurs

## Copies d'écran

![Écran d'accueil](screenshots/welcome_screen.png?raw=true "Écran d'accueil de Collomatique")
![Édition des périodes](screenshots/periods.png?raw=true "Écran d'édition des périodes")
![Édition des modèles de périodicité](screenshots/week_patterns.png?raw=true "Écran d'édition des modèles de périodicité")

## Installation

Avec [Nix](https://nixos.org/download/) (peut être installé sur n'importe quelle distribution Linux) :
```bash
nix-build
./result/bin/collomatique-gtk4

# Ou avec flakes
nix run
```

Sous Ubuntu (testé sur 25.11), il faut d'abord installer Rust (dernière version recommandée) via [rustup](https://rustup.rs), puis :
```bash
sudo apt install build-essential libglib2.0-dev libpango1.0-dev libgdk-pixbuf-2.0-dev libgraphene-1.0-dev libgtk-4-dev libadwaita-1-dev coinor-libcbc-dev coinor-cbc libpython3-dev
cargo build --release
cargo run --release
```
Le mode `--release` est fortement recommandé : le solveur ILP est très lent en mode debug.

Le paquet `coinor-cbc` n'est nécessaire que pour l'exécution des tests.

Malheureusement, adwaita 1.7 est nécessaire et donc Collomatique ne compile pas sur Ubuntu 24.04 (LTS au moment d'écrire).

## Résolution par programmation linéaire

Collomatique modélise la construction du colloscope comme un **programme linéaire en nombres entiers** (ILP — *Integer Linear Programming*).

Le principe :

- **Variables binaires** : chaque décision est représentée par une variable qui vaut 0 ou 1. Par exemple, « l'élève X passe en créneau Y » → 0 (non) ou 1 (oui).
- **Contraintes linéaires** : les règles du colloscope sont encodées sous forme d'inégalités. Par exemple, « chaque élève passe exactement une fois par période » se traduit par une somme de variables égale à 1.
- **Fonction objectif** : une expression linéaire à minimiser (ou maximiser) qui permet d'optimiser l'équilibrage, de respecter les préférences, etc.
- **Solveur** : COIN-CBC explore l'espace des solutions et trouve une affectation optimale, ou prouve qu'aucune solution n'existe.

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

Le projet est un workspace Rust composé des crates suivantes :

| Crate | Rôle |
|---|---|
| `ilp/` | Primitives ILP : expressions linéaires, contraintes, objectifs, variables, construction de problèmes, interface solveur |
| `ilp-modeler/` | Modéliseur ILP générique, indépendant du langage : expansion paresseuse de variables, réification, objectification, composition par bundles |
| `ilp-modeler-derive/` | Macros dérivées pour ilp-modeler (`#[derive(DescribeVar)]`) |
| `constraints-colloscopes/` | Modélisation des contraintes du colloscope en Rust natif : périodicités, appariements, équilibrage, structure d'emploi du temps |
| `state/` et `state-colloscopes/` | Traits et structures d'état de l'application |
| `sqlite-state/` | Persistance SQLite (SQLx) |
| `storage/` | Sérialisation des fichiers (JSON) |
| `gtk4/` | Interface graphique GTK4/Adwaita (Relm4) |
| `python/` | Bindings Python (PyO3), utilisés notamment pour l'import Pronote |
| `rpc/` et `rpc-engine/` | Protocole RPC pour la communication entre processus |
| `time/` | Types pour représenter les jours, heures, etc dans Collomatique |
| `ops/` | Opérations de haut-niveau (GUI et Python) sur l'état de l'application |
| `xlsx/` | Export du colloscope au format xlsx |
| `mps/` | Export de problèmes ILP au format MPS |
