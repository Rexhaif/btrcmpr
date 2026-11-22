# Stored representations on the selected build

Each row is based on the exact archive SHA-256 from the [standalone round trip](verified-cells.csv). The counts below describe the outermost blocks, not guesses from the file name or level. The [per-representation bytes](representation-bytes.csv) show how much input and compressed payload each kind accounts for. Header bytes plus all stored payload bytes equal the full archive. Shared dictionary bytes are already included in the payload.

| File and level | Stored blocks (tag: count) | Framing bytes | Shared dictionary bytes | Row strides (samples: jobs) |
|---|---|---:|---:|---|
| [enwik8, 1](../../native-combined-1/level-01-enwik8.csv) | 1: 256 | 1,044 | 0 | — |
| [mozilla, 1](../../native-combined-1/level-01-mozilla.csv) | 1: 215, 4: 76 | 15,242 | 0 | — |
| [x-ray, 1](../../native-combined-1/level-01-x-ray.csv) | 6: 8 | 52 | 0 | — |
| [enwik8, 2](l02/level-02-enwik8.csv) | 1: 112 | 468 | 0 | — |
| [mozilla, 2](l02/level-02-mozilla.csv) | 1: 235, 4: 76 | 15,382 | 0 | — |
| [x-ray, 2](l02/level-02-x-ray.csv) | 63: 20 | 100 | 0 | 1898: 3, 1899: 7, 1900: 10 |
| [enwik8, 3](l03/level-03-enwik8.csv) | 1: 48 | 212 | 0 | — |
| [mozilla, 3](l03/level-03-mozilla.csv) | 1: 235, 4: 76 | 15,382 | 0 | — |
| [x-ray, 3](l03/level-03-x-ray.csv) | 63: 12 | 68 | 0 | 1898: 2, 1899: 4, 1900: 6 |
| [enwik8, 4](l04/level-04-enwik8.csv) | 1: 48 | 212 | 0 | — |
| [mozilla, 4](l04/level-04-mozilla.csv) | 1: 136, 4: 76, 12: 185 | 15,994 | 0 | — |
| [x-ray, 4](tracks/x-ray/level-04-x-ray.csv) | 63: 12 | 68 | 0 | 1898: 2, 1899: 4, 1900: 6 |
| [enwik8, 5](l05/level-05-enwik8.csv) | 13: 24 | 116 | 0 | — |
| [mozilla, 5](l05/level-05-mozilla.csv) | 1: 110, 4: 14, 9: 47, 12: 148 | 15,448 | 0 | — |
| [x-ray, 5](tracks/x-ray/level-05-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 5, 1900: 3 |
| [enwik8, 6](l06/level-06-enwik8.csv) | 13: 20 | 100 | 0 | — |
| [mozilla, 6](l06/level-06-mozilla.csv) | 1: 156, 4: 15, 8: 1, 9: 70, 12: 211 | 16,386 | 0 | — |
| [x-ray, 6](l06/level-06-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 5, 1900: 3 |
| [enwik8, 7](l07/level-07-enwik8.csv) | 13: 8 | 52 | 0 | — |
| [mozilla, 7](l07/level-07-mozilla.csv) | 1: 136, 4: 14, 9: 62, 12: 185 | 15,994 | 0 | — |
| [x-ray, 7](tracks/x-ray/level-07-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 5, 1900: 3 |
| [enwik8, 8](l08/level-08-enwik8.csv) | 13: 4 | 36 | 0 | — |
| [mozilla, 8](l08/level-08-mozilla.csv) | 1: 122, 4: 10, 8: 1, 9: 57, 12: 164 | 15,694 | 0 | — |
| [x-ray, 8](tracks/x-ray/level-08-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 5, 1900: 3 |
| [enwik8, 9](l09/level-09-enwik8.csv) | 41: 4 | 36 | 0 | — |
| [mozilla, 9](l09/level-09-mozilla.csv) | 1: 117, 4: 9, 9: 56, 12: 156 | 15,580 | 0 | — |
| [x-ray, 9](tracks/x-ray/level-09-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 5, 1900: 3 |
| [enwik8, 10](l10/level-10-enwik8.csv) | 13: 4 | 36 | 0 | — |
| [mozilla, 10](l10/level-10-mozilla.csv) | 1: 110, 4: 5, 9: 56, 12: 148 | 15,448 | 0 | — |
| [x-ray, 10](tracks/x-ray/level-10-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 5, 1900: 3 |
| [enwik8, 11](l11/level-11-enwik8.csv) | 13: 4 | 36 | 0 | — |
| [mozilla, 11](l11/level-11-mozilla.csv) | 1: 105, 4: 3, 9: 55, 12: 141 | 15,345 | 0 | — |
| [x-ray, 11](tracks/x-ray/level-11-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 5, 1900: 3 |
| [enwik8, 12](l12/level-12-enwik8.csv) | 13: 4 | 36 | 0 | — |
| [mozilla, 12](l12/level-12-mozilla.csv) | 1: 100, 4: 5, 9: 51, 12: 135 | 15,253 | 0 | — |
| [x-ray, 12](tracks/x-ray/level-12-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 13](l13/level-13-enwik8.csv) | 48: 4 | 36 | 0 | — |
| [mozilla, 13](l13/level-13-mozilla.csv) | 1: 97, 4: 3, 9: 51, 12: 129 | 15,176 | 0 | — |
| [x-ray, 13](tracks/x-ray/level-13-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 14](l14/level-14-enwik8.csv) | 48: 4 | 36 | 0 | — |
| [mozilla, 14](l14/level-14-mozilla.csv) | 1: 92, 4: 3, 8: 1, 9: 49, 12: 123 | 15,091 | 0 | — |
| [x-ray, 14](tracks/x-ray/level-14-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 15](l15/level-15-enwik8.csv) | 48: 4 | 36 | 0 | — |
| [mozilla, 15](l15/level-15-mozilla.csv) | 1: 89, 4: 5, 8: 1, 9: 44, 12: 118 | 15,016 | 0 | — |
| [x-ray, 15](tracks/x-ray/level-15-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 16](l16/level-16-enwik8.csv) | 50: 2, 51: 2 | 36 | 3,554,991 | — |
| [mozilla, 16](l16/level-16-mozilla.csv) | 1: 67, 4: 1, 9: 35, 12: 87 | 14,546 | 0 | — |
| [x-ray, 16](tracks/x-ray/level-16-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 17](l17/level-17-enwik8.csv) | 50: 1, 51: 3 | 36 | 2,793,019 | — |
| [mozilla, 17](l17/level-17-mozilla.csv) | 1: 36, 4: 6, 9: 70, 21: 68 | 14,475 | 0 | — |
| [x-ray, 17](tracks/x-ray/level-17-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 18](l18/level-18-enwik8.csv) | 50: 1, 51: 3 | 36 | 2,960,211 | — |
| [mozilla, 18](l18/level-18-mozilla.csv) | 1: 29, 9: 17, 21: 37 | 13,798 | 0 | — |
| [x-ray, 18](tracks/x-ray/level-18-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 19](l19/level-19-enwik8.csv) | 50: 1, 51: 3 | 36 | 2,895,202 | — |
| [mozilla, 19](l19/level-19-mozilla.csv) | 1: 29, 8: 1, 9: 17, 21: 36 | 13,798 | 0 | — |
| [x-ray, 19](tracks/x-ray/level-19-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 20](l20/level-20-enwik8.csv) | 50: 1, 51: 3 | 36 | 2,449,010 | — |
| [mozilla, 20](l20/level-20-mozilla.csv) | 1: 28, 8: 1, 9: 17, 21: 36 | 13,791 | 0 | — |
| [x-ray, 20](tracks/x-ray/level-20-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 21](l21/level-21-enwik8.csv) | 50: 1, 51: 3 | 36 | 2,449,010 | — |
| [mozilla, 21](l21/level-21-mozilla.csv) | 1: 28, 9: 17, 21: 34 | 13,770 | 0 | — |
| [x-ray, 21](tracks/x-ray/level-21-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |
| [enwik8, 22](l22/level-22-enwik8.csv) | 50: 1, 51: 3 | 36 | 2,435,154 | — |
| [mozilla, 22](l22/level-22-mozilla.csv) | 1: 41, 9: 24, 21: 53 | 14,043 | 0 | — |
| [x-ray, 22](tracks/x-ray/level-22-x-ray.csv) | 63: 8 | 52 | 0 | 1899: 1, 1900: 7 |

Archive headers available for 66 of 66 measured cells. An omitted row has no header capture here.

| Tag | Stored representation |
|---:|---|
| 1 | LZ |
| 4 | records |
| 6 | adjacent 16-bit predictor |
| 8 | instruction streams |
| 9 | record columns |
| 12 | instruction banks |
| 13 | LZ with extended distances |
| 21 | adaptive instruction banks |
| 41 | paged LZ |
| 48 | local phrase dictionary |
| 50 | shared phrase dictionary and first token stream |
| 51 | shared dictionary token stream |
| 63 | learned 16-bit row predictor |
