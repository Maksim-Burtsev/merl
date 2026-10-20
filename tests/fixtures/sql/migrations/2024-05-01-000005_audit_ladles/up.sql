-- A namesake table in another schema is another table.
CREATE TABLE audit.ladles (id int, depth int, at int);
SELECT l.depth, l.at FROM ladles l;
--       ^ d: migrations/2024-04-01-000004_pour_strainers/up.sql:6
--                ^ d: none
SELECT a.at FROM audit.ladles a;
--       ^ d: migrations/2024-05-01-000005_audit_ladles/up.sql:2
