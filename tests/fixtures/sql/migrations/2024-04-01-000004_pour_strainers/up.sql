-- #1063: a column through its alias or its table, in the version in effect.
ALTER TABLE strainers ADD COLUMN mesh int;
CREATE TABLE ladles (
    id int,
    strainer_id int REFERENCES strainers (id),
    "depth" int
);
SELECT s.id, s.mesh, l.depth, l.strainer_id FROM strainers s JOIN ladles l ON l.strainer_id = s.id;
--       ^ d: migrations/2024-02-01-000002_widen_strainers/up.sql:3
--             ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:2
--                     ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:6
--                              ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:5
ALTER TABLE strainers RENAME COLUMN mesh TO weave;
SELECT strainers.weave FROM strainers;
--               ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:13
SELECT s.id FROM (SELECT id FROM ladles) s;
--       ^ d: none
WITH ladles AS (SELECT 1 AS id)
SELECT ladles.id FROM ladles;
--            ^ d: none
SELECT x.id FROM strainers s;
--       ^ d: none
INSERT INTO ladles (depth) VALUES (1);
--                  ^ d: none; want migrations/2024-04-01-000004_pour_strainers/up.sql:6 (#1063)
