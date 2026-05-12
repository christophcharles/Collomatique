# Requests CSV Specification

The requests CSV file describes the scheduling requests to fill. See [ROOMS_SPEC.md](ROOMS_SPEC.md) for the rooms CSV specification.

The file must be UTF-8 encoded, comma-delimited, with a header row. Columns must appear in the exact order listed below. No extra columns are allowed.

## Requests CSV

Each row describes one interrogation slot that needs a room assigned to it.

| # | Column | Type | Description |
|---|--------|------|-------------|
| 1 | P1 | 0 or 1 | Whether this interrogation takes place during period 1. |
| 2 | P2 | 0 or 1 | Whether this interrogation takes place during period 2. |
| 3 | P3 | 0 or 1 | Whether this interrogation takes place during period 3. |
| 4 | Jour | weekday name | Day of the week, in French: Lundi, Mardi, Mercredi, Jeudi, Vendredi, Samedi. |
| 5 | Heure | integer 8–19 | Hour at which the interrogation starts. Must be between 8 and 19 inclusive. |
| 6 | Discipline | subject name | Subject being taught. Must match exactly one of the allowed subject names (see below). The value is NFC-normalized before comparison; matching is case-sensitive. |
| 7 | Classes | semicolon-separated list | One or more class names separated by semicolons. Each name must be from the allowed class list (see below). At least one class is required. Whitespace around each name is trimmed. |
| 8 | Responsable | non-empty string | Name of the person requesting the room (typically the academic coordinator for the subject). |
| 9 | Colleur | non-empty string | Name of the teacher who will conduct the interrogation. |
| 10 | Tableaux | integer ≥ 0 | Minimum number of blackboards needed in the assigned room. |
| 11 | Fenêtre | 0 or 1 | Whether a window is required. 1 = required, 0 = no preference. |
| 12 | Nb élèves | integer ≥ 1 | Number of students who need to fit in the assigned room. Must be strictly positive. |
| 13 | Nb prep | integer ≥ 0 | Number of students who need to fit in a prep room for this slot. 0 means no prep room is needed. |
| 14 | Salle | string or empty | Suggested room for the interrogation. If empty, no preference. If set, must match a room name from the rooms CSV. To request an unmanaged room, list it in the rooms CSV with Priorité = -1; the requester vouches for the room being available at the corresponding time slot. |
| 15 | Prep | string or empty | Suggested prep room. Same semantics as the Salle column. |

### Example

```csv
P1,P2,P3,Jour,Heure,Discipline,Classes,Responsable,Colleur,Tableaux,Fenêtre,Nb élèves,Nb prep,Salle,Prep
1,0,1,Lundi,8,Mathématiques,MP;PC,Dupont,Martin,1,0,3,2,A101,
0,1,1,Mardi,14,Physique,BCPST 1;BCPST 2,Durand,Bernard,2,1,5,0,,
```

## Allowed subject names

Subject names are case-sensitive. The value in the CSV is NFC-normalized (Unicode canonical decomposition followed by canonical composition) before matching, so that equivalent Unicode representations of accented characters are treated identically.

- Mathématiques
- Physique
- Chimie
- Physique-Chimie
- Sciences de l'ingénieur
- Sciences de la Vie et de la Terre
- Informatique
- Français
- Lettres
- Philosophie
- Lettres-Philosophie
- Histoire
- Géographie
- Histoire-Géographie-Géopolitique
- Économie, Sociologie et Histoire du monde contemporain
- Anglais
- Espagnol
- Allemand
- Italien
- Latin
- Grec

## Allowed class names

Class names are case-sensitive and matched after trimming whitespace.

| Name | Description |
|------|-------------|
| MPSI | Mathématiques, Physique et Sciences de l'Ingénieur (1re année) |
| MP2I | Mathématiques, Physique, Ingénierie et Informatique (1re année) |
| MP | Mathématiques-Physique (2e année) |
| MPI | Mathématiques-Physique-Informatique (2e année) |
| MP* | Mathématiques-Physique étoile (2e année) |
| MPI* | Mathématiques-Physique-Informatique étoile (2e année) |
| PCSI 1 | Physique, Chimie et Sciences de l'Ingénieur, groupe 1 (1re année) |
| PCSI 2 | Physique, Chimie et Sciences de l'Ingénieur, groupe 2 (1re année) |
| PC | Physique-Chimie (2e année) |
| PC* | Physique-Chimie étoile (2e année) |
| PCC | Physique-Chimie-Chimie (2e année) |
| BCPST 1 | Biologie, Chimie, Physique et Sciences de la Terre, groupe 1 |
| BCPST 2 | Biologie, Chimie, Physique et Sciences de la Terre, groupe 2 |
| ECG 1A | École de Commerce, voie Générale, 1re année groupe A |
| ECG 1B | École de Commerce, voie Générale, 1re année groupe B |
| ECG 2A | École de Commerce, voie Générale, 2e année groupe A |
| ECG 2B | École de Commerce, voie Générale, 2e année groupe B |
| LS 1 | Lettres et Sciences, groupe 1 |
| LS 2 | Lettres et Sciences, groupe 2 |
