# Copies d'écran

Toutes les copies d'écran ci-dessous ont été prises avec le fichier d'exemple
[`hogwarts.collomatique`](../examples), qui contient un colloscope complet
(3 périodes, 40 semaines, 8 matières, 24 élèves).

Elles sont présentées dans l'ordre des panneaux de l'application.

## Écran d'accueil

![Écran d'accueil](welcome_screen.png?raw=true)

## Planning général

Découpage de l'année en périodes et en semaines. Chaque semaine peut être activée
ou désactivée (vacances, concours blancs...) et recevoir un libellé.

![Planning général](general_planning.png?raw=true)

## Matières

Pour chaque matière : taille des groupes, nombre de groupes par colle, périodicité
et périodes concernées.

![Matières](subjects.png?raw=true)

## Colleurs

![Colleurs](teachers.png?raw=true)

## Modèles de périodicité

Modèles réutilisables (semaines paires, semaines impaires, blocs arbitraires...)
que l'on applique ensuite aux créneaux.

![Modèles de périodicité](periodicity.png?raw=true)

## Créneaux de colles

![Créneaux de colles](slots.png?raw=true)

## Appariements de créneaux

Un créneau peut n'être ouvert que si un autre est utilisé — par exemple pour un
colleur qui n'accepte une deuxième heure que s'il a déjà fait la première.

![Appariements de créneaux](pairing_rules.png?raw=true)

## Incompatibilités horaires

Activités qui empêchent de placer une colle : entraînements, déjeuner, TP...

![Incompatibilités horaires](incompats.png?raw=true)

## Élèves

![Élèves](students.png?raw=true)

## Inscriptions dans les matières

Quels élèves suivent quelles matières, période par période.

![Inscriptions dans les matières](assignments.png?raw=true)

## Groupes de colles

Listes de groupes (une par matière ou partagées entre matières), remplies à la main
ou générées automatiquement.

![Groupes de colles](grouplists.png?raw=true)

## Paramètres par élève

Nombre de colles par semaine et par jour, globalement ou pour un élève donné.

![Paramètres par élève](limits.png?raw=true)

## Équilibrage des colles

Rotation des colleurs et des créneaux, en version souple ou stricte, globalement ou
matière par matière.

![Équilibrage des colles](balancing.png?raw=true)

## Colloscope

Le colloscope lui-même : une ligne par créneau, une colonne par semaine. Il peut être
modifié à la main ou construit automatiquement.

![Colloscope](colloscope.png?raw=true)

## Configuration de la résolution

Choix des périodes à recalculer, réutilisation ou non du colloscope actuel, et réglages
du solveur.

![Configuration de la résolution](solver_config.png?raw=true)

## Résolution en cours

Suivi du solveur : meilleur coût trouvé, meilleur coût encore possible, temps écoulé.

![Résolution en cours](solver_running.png?raw=true)

## Exporter

Export xlsx : choix des feuilles à inclure, couleurs et mise en page configurables.

![Exporter](export.png?raw=true)

## Outils avancés

Statistiques du document, exécution de scripts Python, export du problème ILP au
format MPS.

![Outils avancés](advanced.png?raw=true)
