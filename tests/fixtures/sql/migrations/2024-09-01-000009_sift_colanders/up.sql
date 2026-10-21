-- #1063: a column of a renamed table, and of a table CREATE TABLE ... AS makes.
CREATE TABLE colanders (
    id int,
    holes int
);
ALTER TABLE colanders RENAME TO sifters;
SELECT f.holes FROM sifters f;
--       ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:4
SELECT holes FROM sifters;
--     ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:4
ALTER TABLE sifters ADD COLUMN rim int;
SELECT f.rim, f.holes FROM sifters f;
--       ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:11
--              ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:4
CREATE TABLE meshes AS
SELECT
    max(holes) AS widest,
    f.id,
    rim
FROM (SELECT id, holes, rim FROM sifters) AS f
GROUP BY f.id, rim;
SELECT v.widest, v.id, v.rim, v.holes FROM meshes v;
--       ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:17
--                 ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:18
--                       ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:19
--                              ^ d: none
CREATE TABLE strains AS (SELECT holes, rim widest_rim FROM sifters);
SELECT s.rim FROM strains s;
--       ^ d: none
