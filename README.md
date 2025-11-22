# Collomatique

Programme de construction automatique de colloscopes

## Installation

Avec Nix, on peut compiler avec :
```bash
nix-build
```
Et `nix-run` pour exécuter.

Sous Ubuntu (25.04 et compatibles) :
```
sudo apt install libglib2.0-dev libpango1.0-dev libgdk-pixbuf-2.0-dev libgraphene-1.0-dev libgtk-4-dev libadwaita-1-dev coinor-libcbc-dev libpython3-dev
cargo build
cargo run
```
Malheureusement, adwaita 1.7 est nécessaire et donc Collomatique ne compile pas sur Ubuntu 24.04 (LTS au moment d'écrire).

## Copies d'écran

![Écran d'accueil](screenshots/welcome_screen.png?raw=true "Écran d'accueil de Collomatique")
![Édition des périodes](screenshots/periods.png?raw=true "Écran d'édition des périodes")
![Édition des modèles de périodicité](screenshots/week_patterns.png?raw=true "Écran d'édition des modèles de périodicité")

