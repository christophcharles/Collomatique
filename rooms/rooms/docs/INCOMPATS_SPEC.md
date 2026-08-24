# Incompats CSV Specification

The incompats CSV file declares room incompatibilities — times when a room cannot be used. This file is optional. See [ROOMS_SPEC.md](ROOMS_SPEC.md) for the rooms CSV specification and [REQUESTS_SPEC.md](REQUESTS_SPEC.md) for the requests CSV specification.

The file must be UTF-8 encoded, comma-delimited, with a header row. Columns must appear in the exact order listed below. No extra columns are allowed.

Each row declares that a room is unavailable at a given day and hour for one or more periods.

| # | Column | Type | Description |
|---|--------|------|-------------|
| 1 | Salle | non-empty string | Name of the room. Must match a room declared in the rooms CSV file. |
| 2 | P1 | 0 or 1 | Whether the incompatibility applies during period 1. |
| 3 | P2 | 0 or 1 | Whether the incompatibility applies during period 2. |
| 4 | P3 | 0 or 1 | Whether the incompatibility applies during period 3. |
| 5 | Jour | weekday name | Day of the week, in French: Lundi, Mardi, Mercredi, Jeudi, Vendredi, Samedi. |
| 6 | Heure | integer 8–19 | Hour at which the incompatibility applies. Must be between 8 and 19 inclusive. |

## Example

```csv
Salle,P1,P2,P3,Jour,Heure
A101,1,0,1,Lundi,8
B203,0,1,0,Mardi,14
```
