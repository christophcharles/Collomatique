# Rooms CSV Specification

The rooms CSV file describes the available rooms that can be assigned to requests.

The file must be UTF-8 encoded, comma-delimited, with a header row. Columns must appear in the exact order listed below. No extra columns are allowed.

Each row describes one room that can be assigned to requests.

| # | Column | Type | Description |
|---|--------|------|-------------|
| 1 | Salle | non-empty string | Unique name identifying the room (e.g. "A101", "Labo Chimie 2"). |
| 2 | Étage | integer ≥ 0 | Floor number where the room is located. Ground floor is 0. |
| 3 | X | decimal | X coordinate of the room on the floor plan. Used to estimate walking distance between rooms. |
| 4 | Y | decimal | Y coordinate of the room on the floor plan. Used to estimate walking distance between rooms. |
| 5 | Tableaux noirs | integer ≥ 0 | Number of blackboards available in the room. |
| 6 | Tableaux blancs | integer ≥ 0 | Number of whiteboards available in the room. |
| 7 | Capacité | integer ≥ 1 | Maximum number of students the room can seat. Must be strictly positive. |
| 8 | Fenêtre | Non, Intérieur, or Extérieur | Type of window in the room. Non = no window, Intérieur = interior-facing window, Extérieur = exterior-facing window. |
| 9 | Priorité | integer ≥ 0 or -1 | Priority rank for room selection. 0 = use first, 1 = use second, etc. -1 = never use unless explicitly requested (see below). |
| 10 | Réservée | 0 or 1 | Whether this room is reserved for oral exam preparation. 0 = no, 1 = yes. |

### Unmanaged rooms (Priorité = -1)

Rooms with Priorité = -1 are never assigned automatically. They can only be used when a request explicitly names them in its Salle or Prep column. The requester vouches for the room being available at the corresponding time slot.

## Example

```csv
Salle,Étage,X,Y,Tableaux noirs,Tableaux blancs,Capacité,Fenêtre,Priorité,Réservée
A101,1,2.5,3.0,2,1,30,Extérieur,0,0
B203,2,10.0,5.5,1,0,15,Non,2,1
C001,0,0.0,1.0,0,0,10,Intérieur,-1,0
```
