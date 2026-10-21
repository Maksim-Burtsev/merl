-- #1063: a bare column where SQL names one table.
ALTER TABLE ladles
    ADD COLUMN spout int,
    ADD COLUMN rim int;
INSERT INTO ladles (depth, spout) VALUES (1, 2);
--                  ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:6
--                         ^ d: migrations/2024-08-01-000008_fill_ladles/up.sql:3
INSERT INTO ladles (depth) SELECT depth FROM strainers;
--                                ^ d: none
UPDATE ladles SET rim = rim + 1 WHERE depth = 0;
--                ^ d: migrations/2024-08-01-000008_fill_ladles/up.sql:4
--                      ^ d: migrations/2024-08-01-000008_fill_ladles/up.sql:4
--                                    ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:6
UPDATE ladles l SET rim = s.id FROM strainers s WHERE depth = s.id;
--                  ^ d: migrations/2024-08-01-000008_fill_ladles/up.sql:4
--                                                    ^ d: none
SELECT depth FROM ladles WHERE spout > 0;
--     ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:6
--                             ^ d: migrations/2024-08-01-000008_fill_ladles/up.sql:3
SELECT depth FROM ladles JOIN strainers ON true;
--     ^ d: none
SELECT id, (SELECT weave FROM strainers) FROM ladles;
--     ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:4
--                 ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:13
SELECT rim AS depth FROM ladles ORDER BY depth;
--                                       ^ d: none
SELECT spout FROM (SELECT spout FROM ladles) d;
--     ^ d: none
SELECT depth FROM ladles, generate_series(1, 2) g;
--     ^ d: none
ALTER TABLE ladles RENAME COLUMN rim TO brim;
--                               ^ d: migrations/2024-08-01-000008_fill_ladles/up.sql:4
ALTER TABLE ladles DROP COLUMN brim;
--                             ^ d: migrations/2024-08-01-000008_fill_ladles/up.sql:31
ALTER TABLE ladles ADD CONSTRAINT deep CHECK (depth > 0);
--                                            ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:6
ALTER TABLE ladles ADD FOREIGN KEY (spout) REFERENCES strainers (id);
--                                                               ^ d: none
CREATE TRIGGER ladle_depth AFTER UPDATE OF depth ON ladles FOR EACH ROW EXECUTE FUNCTION drain ();
--                                         ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:6
