-- #1126: a column inherited or copied, an output name, a quoted name, a conflict's target.
SELECT handle, spout, brim FROM urns;
--     ^ d: steep/schema.sql:9
--             ^ d: steep/schema.sql:4
--                    ^ d: steep/schema.sql:5
SELECT knit, brim FROM cozies;
--     ^ d: steep/schema.sql:12
--           ^ d: steep/schema.sql:5
SELECT strength FROM brewings;
--     ^ d: none
SELECT spout * 2 doubled FROM samovars ORDER BY doubled;
--                                              ^ d: steep/pour.sql:11
SELECT spout * 2 id FROM samovars ORDER BY id;
--               ^ d: none
--                                         ^ d: steep/pour.sql:13
SELECT CASE WHEN spout > 0 THEN 1 ELSE 0 END brim FROM samovars ORDER BY brim;
--                                                                       ^ d: steep/pour.sql:16
SELECT DISTINCT spout FROM samovars ORDER BY spout;
--                                           ^ d: steep/schema.sql:4
SELECT spout * 2 AS id FROM samovars ORDER BY id;
--                                            ^ d: steep/pour.sql:20
SELECT id FROM ONLY samovars;
--     ^ d: steep/schema.sql:3
SELECT s.spout FROM ONLY samovars s;
--       ^ d: steep/schema.sql:4
SELECT brim FROM samovars TABLESAMPLE SYSTEM (10);
--     ^ d: steep/schema.sql:5
SELECT x FROM samovars AS s (x, y);
--     ^ d: steep/pour.sql:28
INSERT INTO samovars (id, spout) VALUES (1, 2)
ON CONFLICT (id) DO UPDATE SET spout = EXCLUDED.spout
--           ^ d: steep/schema.sql:3
--                             ^ d: steep/schema.sql:4
--                                              ^ d: steep/schema.sql:4
RETURNING brim;
--        ^ d: steep/schema.sql:5
CREATE INDEX ON urns (handle);
--                    ^ d: steep/schema.sql:9
COMMENT ON COLUMN samovars.brim IS 'the top';
--                         ^ d: steep/schema.sql:5
WITH samovars AS (SELECT 1 AS spout) SELECT spout FROM samovars;
--                                          ^ d: steep/pour.sql:41
SELECT sum(spout) OVER w FROM samovars WINDOW w AS (ORDER BY id);
--                     ^ d: steep/pour.sql:43
--                                                           ^ d: steep/schema.sql:3
SELECT "Lid", lid FROM "Kettle";
--      ^ d: steep/schema.sql:17
--            ^ d: steep/schema.sql:18
SELECT * FROM phantoms, mirages;
--            ^ d: none
--                      ^ d: none
SELECT id, nothere FROM cozies;
--     ^ d: steep/schema.sql:3
--         ^ d: none
INSERT INTO samovars (id) VALUES (1) ON CONFLICT ON CONSTRAINT brim DO NOTHING RETURNING spout AS handle;
--                                                             ^ d: none
--                                                                                       ^ d: steep/schema.sql:4
--                                                                                                ^ d: none
INSERT INTO urns (id) VALUES (1) RETURNING (SELECT max(handle) FROM samovars);
--                                                     ^ d: none
CREATE UNIQUE INDEX spout_idx ON samovars USING btree (lower(brim));
--                  ^ d: steep/pour.sql:61
--                                                           ^ d: steep/schema.sql:5
SELECT LID, "lid", k."Lid" FROM "Kettle" k;
--     ^ d: steep/schema.sql:18
--           ^ d: steep/schema.sql:18
--                    ^ d: steep/schema.sql:17
SELECT * FROM crumbs, scones;
--            ^ d: steep/seed.mysql:3
--                    ^ d: steep/dump.sql:3
SELECT lid FROM /* A block comment nests: /* this one closes first */ "Kettle", */ samovars;
--     ^ d: none
-- #1228: only a bare name in the ORDER BY of the same query is an output name; a CTE body reads the table.
SELECT spout * 2 AS doubled FROM samovars ORDER BY doubled + 1;
--                                                 ^ d: none
SELECT sum(spout) OVER (w ORDER BY spout) FROM samovars WINDOW w AS (PARTITION BY brim);
--                      ^ d: steep/pour.sql:76
--                                 ^ d: steep/schema.sql:4
WITH samovars AS (SELECT spout AS spout FROM samovars) SELECT spout FROM samovars;
--                       ^ d: steep/schema.sql:4
--                                                            ^ d: steep/pour.sql:79
WITH cups (sip) AS (SELECT spout FROM samovars) SELECT sip FROM cups ORDER BY sip;
--                                                     ^ d: steep/pour.sql:82
SELECT TOP 5 spout FROM samovars ORDER BY spout;
--                                        ^ d: steep/schema.sql:4
SELECT spout FROM samovars WHERE id IN (WITH samovars AS (SELECT 1 AS spout) SELECT spout FROM samovars);
--     ^ d: steep/schema.sql:4
--                                                                                  ^ d: steep/pour.sql:86
-- #1255's review: a CTE or a column alias list answers only in the SELECT that reads it; a select modifier is no expression.
WITH mugs AS (SELECT 1 AS spout) INSERT INTO samovars (spout) SELECT spout FROM mugs;
--                                                     ^ d: steep/schema.sql:4
--                                                                   ^ d: steep/pour.sql:90
INSERT INTO samovars (spout) SELECT spout FROM urns AS u (spout);
--                    ^ d: steep/schema.sql:4
--                                  ^ d: steep/pour.sql:93
WITH mugs AS (SELECT 1 AS brim) SELECT brim FROM mugs UNION SELECT brim FROM samovars;
--                                     ^ d: steep/pour.sql:96
--                                                                 ^ d: steep/schema.sql:5
SELECT SQL_NO_CACHE spout FROM samovars ORDER BY spout;
--                  ^ d: steep/schema.sql:4
--                                               ^ d: steep/schema.sql:4
SELECT HIGH_PRIORITY STRAIGHT_JOIN SQL_CALC_FOUND_ROWS spout FROM samovars ORDER BY spout;
--                                                                                  ^ d: steep/schema.sql:4
SELECT DISTINCT ON (brim) spout FROM samovars ORDER BY spout;
--                                                     ^ d: steep/schema.sql:4
SELECT ALL spout FROM samovars ORDER BY spout;
--                                      ^ d: steep/schema.sql:4
SELECT TOP 5 WITH TIES spout FROM samovars ORDER BY spout;
--                                                  ^ d: steep/schema.sql:4
SELECT SQL_NO_CACHE spout * 2 doubled FROM samovars ORDER BY doubled;
--                                                           ^ d: steep/pour.sql:110
WITH infusions AS (SELECT 1 AS spout)
/* a nested comment: /* ( */ ; */
SELECT spout FROM infusions;
--                ^ d: steep/pour.sql:112
SELECT * FROM muffins, crusts;
--            ^ d: steep/bake.mysql:3
--                     ^ d: steep/crust.mysql:3
-- #1260: a CTE answers only in its scope, a UNION branch reads only its own FROM; the shapes around them.
WITH samovars AS (SELECT 1 AS brim) SELECT brim FROM urns UNION SELECT brim FROM samovars;
--                                                                     ^ d: steep/pour.sql:120
SELECT lid FROM "Kettle" UNION SELECT lid FROM samovars;
--     ^ d: steep/schema.sql:18
--                                    ^ d: none
SELECT spout FROM urns UNION SELECT spout FROM samovars ORDER BY spout;
--                                                               ^ d: none
WITH RECURSIVE samovars AS (SELECT spout FROM samovars) SELECT 1;
--                                 ^ d: none
SELECT spout FROM samovars CROSS APPLY (SELECT 1 AS spout) a;
--     ^ d: none
WITH mugs AS (SELECT spout FROM samovars), samovars AS (SELECT 1 AS spout) SELECT spout FROM mugs;
--                   ^ d: steep/schema.sql:4
SELECT CAST(spout AS brim) FROM samovars;
--                   ^ d: none
SELECT spout FROM samovars WHERE id IN (SELECT 1 AS spout);
--     ^ d: none
SELECT spout FROM samovars WHERE id IN (WITH RECURSIVE samovars AS (SELECT 1 AS spout) SELECT spout FROM samovars);
--     ^ d: steep/schema.sql:4
-- #1357's review: a CTE that reads its own name in a UNION branch is recursive without the keyword (SQLite, T-SQL).
WITH samovars (spout) AS (SELECT 1 UNION ALL SELECT spout + 1 FROM samovars WHERE spout < 9) SELECT spout FROM samovars;
--                                                  ^ d: none
--                                                                                ^ d: none
WITH samovars AS (SELECT 1 AS spout UNION ALL SELECT spout + 1 FROM samovars WHERE spout < 9) SELECT spout FROM samovars;
--                                                   ^ d: none
--                                                                                 ^ d: none
