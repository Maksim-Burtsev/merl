-- #1064: outside a migration, every version.
SELECT * FROM strainers;
--            ^ d: picker migrations/2024-01-01-000001_create_strainers/up.sql:2, migrations/2024-02-01-000002_widen_strainers/down.sql:3, migrations/2024-02-01-000002_widen_strainers/up.sql:3
SELECT s.id FROM strainers s;
--       ^ d: picker migrations/2024-01-01-000001_create_strainers/up.sql:2, migrations/2024-02-01-000002_widen_strainers/up.sql:3
